use std::fmt::{self, Write as _};
use std::io::IsTerminal;

use nu_ansi_term::{Color, Style};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::field::RecordFields;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::{ChronoLocal, FormatTime};
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields, FormattedFields};
use tracing_subscriber::registry::LookupSpan;

const LARGEUR_NIVEAU: usize = 5;
const LARGEUR_CIBLE: usize = 18;

/// Formateur d'événements `tracing` pensé pour la lecture en développement.
///
/// Rend une ligne par événement : `HH:MM:SS  NIVEAU  cible  message  clé=valeur`.
///
/// Le même type sert de formateur de champs. Poser les deux ensemble
/// (`.event_format(…).fmt_fields(…)`) est ce qui garantit que les champs hérités
/// d'un span suivent la même convention et le même choix de couleur.
pub struct PrettyFormat {
    ansi: bool,
    horodatage: ChronoLocal,
}

impl PrettyFormat {
    /// Construit le formateur, la couleur suivant que la sortie standard est un terminal.
    pub fn new() -> Self {
        Self::with_ansi(std::io::stdout().is_terminal())
    }

    /// Construit le formateur en imposant l'usage de la couleur.
    pub fn with_ansi(ansi: bool) -> Self {
        Self {
            ansi,
            horodatage: ChronoLocal::new("%H:%M:%S".to_owned()),
        }
    }

    fn peindre(&self, style: Style, texte: &str) -> String {
        if self.ansi {
            style.paint(texte).to_string()
        } else {
            texte.to_owned()
        }
    }

    fn style_du_niveau(niveau: &Level) -> Style {
        match *niveau {
            Level::TRACE => Color::DarkGray.into(),
            Level::DEBUG => Color::Blue.into(),
            Level::INFO => Color::Green.into(),
            Level::WARN => Color::Yellow.into(),
            Level::ERROR => Color::Red.into(),
        }
    }
}

impl Default for PrettyFormat {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, N> FormatEvent<S, N> for PrettyFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadonnees = event.metadata();

        self.horodatage.format_time(&mut writer)?;

        // La couleur est appliquée après l'alignement : les séquences ANSI comptent
        // dans la largeur demandée à `format!` et décaleraient les colonnes.
        let niveau = format!("{:<LARGEUR_NIVEAU$}", metadonnees.level().as_str());
        let style = Self::style_du_niveau(metadonnees.level());
        write!(writer, "  {}", self.peindre(style, &niveau))?;

        let cible = format!("{:<LARGEUR_CIBLE$}", metadonnees.target());
        write!(writer, "  {}", self.peindre(Style::new().dimmed(), &cible))?;

        let mut visiteur = ChampsEvenement::default();
        event.record(&mut visiteur);
        write!(writer, "  {}", visiteur.message)?;

        // Les champs des spans parents suivent ceux de l'événement : sans eux, le
        // `request_id` que le middleware attache au span ne serait jamais journalisé.
        // Ils arrivent déjà peints par l'implémentation de `FormatFields` ci-dessous.
        let mut champs = Vec::new();
        if !visiteur.champs.is_empty() {
            champs.push(self.peindre(Style::new().dimmed(), &visiteur.champs));
        }
        if let Some(portee) = ctx.event_scope() {
            for span in portee {
                let extensions = span.extensions();
                let Some(formates) = extensions.get::<FormattedFields<N>>() else {
                    continue;
                };
                if !formates.fields.is_empty() {
                    champs.push(formates.fields.clone());
                }
            }
        }

        if !champs.is_empty() {
            write!(writer, "  {}", champs.join(" "))?;
        }

        writeln!(writer)
    }
}

impl<'writer> FormatFields<'writer> for PrettyFormat {
    fn format_fields<R: RecordFields>(
        &self,
        mut writer: Writer<'writer>,
        fields: R,
    ) -> fmt::Result {
        let mut visiteur = ChampsEvenement::default();
        fields.record(&mut visiteur);

        if visiteur.champs.is_empty() {
            return Ok(());
        }
        write!(
            writer,
            "{}",
            self.peindre(Style::new().dimmed(), &visiteur.champs)
        )
    }
}

#[derive(Default)]
struct ChampsEvenement {
    message: String,
    champs: String,
}

impl ChampsEvenement {
    fn ajouter(&mut self, champ: &Field, valeur: fmt::Arguments<'_>) {
        if champ.name() == "message" {
            let _ = write!(self.message, "{valeur}");
            return;
        }

        if !self.champs.is_empty() {
            self.champs.push(' ');
        }
        let _ = write!(self.champs, "{}={}", champ.name(), valeur);
    }
}

impl Visit for ChampsEvenement {
    fn record_str(&mut self, champ: &Field, valeur: &str) {
        self.ajouter(champ, format_args!("{valeur}"));
    }

    fn record_debug(&mut self, champ: &Field, valeur: &dyn fmt::Debug) {
        self.ajouter(champ, format_args!("{valeur:?}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct Tampon(Arc<Mutex<Vec<u8>>>);

    impl Tampon {
        fn contenu(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    impl io::Write for Tampon {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for Tampon {
        type Writer = Tampon;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn capture(format: PrettyFormat, emettre: impl FnOnce()) -> String {
        let tampon = Tampon::default();
        let champs = PrettyFormat::with_ansi(format.ansi);
        let abonne = tracing_subscriber::fmt()
            .fmt_fields(champs)
            .event_format(format)
            .with_writer(tampon.clone())
            .with_max_level(tracing::Level::TRACE)
            .finish();
        tracing::subscriber::with_default(abonne, emettre);
        tampon.contenu()
    }

    #[test]
    fn aucune_couleur_quand_la_sortie_n_est_pas_un_tty() {
        let sortie = capture(PrettyFormat::new(), || {
            let span = tracing::info_span!("requete", request_id = "01JQ3F8K2P");
            let _entree = span.enter();
            tracing::info!(status = 200, "bonjour");
        });

        assert!(
            !sortie.contains('\u{1b}'),
            "sortie colorée hors TTY : {sortie:?}"
        );
    }

    #[test]
    fn les_couleurs_sont_presentes_quand_elles_sont_forcees() {
        let sortie = capture(PrettyFormat::with_ansi(true), || tracing::info!("bonjour"));

        assert!(
            sortie.contains('\u{1b}'),
            "aucune séquence ANSI : {sortie:?}"
        );
    }

    #[test]
    fn la_ligne_porte_le_niveau_la_cible_le_message_puis_les_champs() {
        let sortie = capture(PrettyFormat::with_ansi(false), || {
            tracing::warn!(actives = 18, max = 20, "pool proche de la saturation")
        });

        let niveau = sortie.find("WARN").expect("niveau absent");
        let cible = sortie
            .find("rbs_core::logs::pretty")
            .expect("cible absente");
        let message = sortie
            .find("pool proche de la saturation")
            .expect("message absent");
        let champs = sortie.find("actives=18").expect("champs absents");

        assert!(
            niveau < cible && cible < message && message < champs,
            "ordre inattendu : {sortie:?}"
        );
        assert!(sortie.contains("max=20"), "champ manquant : {sortie:?}");
    }

    #[test]
    fn les_champs_d_un_span_parent_sont_repris_apres_ceux_de_l_evenement() {
        let sortie = capture(PrettyFormat::with_ansi(false), || {
            let span = tracing::info_span!("requete", request_id = "01JQ3F8K2P");
            let _entree = span.enter();
            tracing::error!(status = 422, "requête refusée");
        });

        let champ_evenement = sortie.find("status=422").expect("champ d'événement absent");
        let champ_span = sortie
            .find("request_id=01JQ3F8K2P")
            .expect("champ de span absent");

        assert!(champ_evenement < champ_span, "ordre inattendu : {sortie:?}");
        assert!(
            !sortie.contains('"'),
            "valeurs entre guillemets : {sortie:?}"
        );
    }

    #[test]
    fn les_cinq_niveaux_sont_rendus_avec_leur_libelle() {
        let sortie = capture(PrettyFormat::with_ansi(false), || {
            tracing::trace!("t");
            tracing::debug!("d");
            tracing::info!("i");
            tracing::warn!("w");
            tracing::error!("e");
        });

        for niveau in ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"] {
            assert!(
                sortie.contains(niveau),
                "niveau {niveau} absent : {sortie:?}"
            );
        }
    }
}
