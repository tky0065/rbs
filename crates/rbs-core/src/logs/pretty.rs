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
/// Rend une ligne par événement : `HH:MM:SS  NIVEAU  target  message  clé=value`.
///
/// Le même type sert de formateur de champs. Poser les deux ensemble
/// (`.event_format(…).fmt_fields(…)`) est ce qui garantit que les champs hérités
/// d'un span suivent la même convention et le même choix de couleur.
#[non_exhaustive]
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

    /// Le style à appliquer, neutre quand la couleur est coupée.
    ///
    /// Rendre le style plutôt que le texte peint évite d'allouer : `prefix` et `suffix`
    /// sont des `Display` qui écrivent directement dans le flux, et n'écrivent rien du
    /// tout pour un style vide. Le formateur est traversé à chaque événement tracé,
    /// c'est-à-dire à chaque requête servie.
    fn style(&self, style: Style) -> Style {
        if self.ansi { style } else { Style::new() }
    }

    fn level_style(level: &Level) -> Style {
        match *level {
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
        let metadata = event.metadata();

        self.horodatage.format_time(&mut writer)?;

        // Les séquences ANSI encadrent le champ déjà aligné : à l'intérieur, elles
        // compteraient dans la largeur demandée et décaleraient les colonnes.
        let niveau = self.style(Self::level_style(metadata.level()));
        write!(
            writer,
            "  {}{:<LARGEUR_NIVEAU$}{}",
            niveau.prefix(),
            metadata.level().as_str(),
            niveau.suffix()
        )?;

        let cible = self.style(Style::new().dimmed());
        write!(
            writer,
            "  {}{:<LARGEUR_CIBLE$}{}",
            cible.prefix(),
            metadata.target(),
            cible.suffix()
        )?;

        let mut visiteur = ChampsEvenement::default();
        event.record(&mut visiteur);
        write!(writer, "  {}", visiteur.message)?;

        // Les champs des spans parents suivent ceux de l'événement : sans eux, le
        // `request_id` que le middleware attache au span ne serait jamais journalisé.
        // Ils arrivent déjà peints par l'implémentation de `FormatFields` ci-dessous.
        //
        // Ils sont écrits à la file plutôt que rassemblés : le `Vec` que cela demandait
        // clonait chaque bloc de champs pour le rejoindre aussitôt.
        let mut separateur = "  ";
        if !visiteur.fields.is_empty() {
            let champs = self.style(Style::new().dimmed());
            write!(
                writer,
                "{separateur}{}{}{}",
                champs.prefix(),
                visiteur.fields,
                champs.suffix()
            )?;
            separateur = " ";
        }
        if let Some(portee) = ctx.event_scope() {
            for span in portee {
                let extensions = span.extensions();
                let Some(formates) = extensions.get::<FormattedFields<N>>() else {
                    continue;
                };
                if !formates.fields.is_empty() {
                    write!(writer, "{separateur}{}", formates.fields)?;
                    separateur = " ";
                }
            }
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

        if visiteur.fields.is_empty() {
            return Ok(());
        }
        let champs = self.style(Style::new().dimmed());
        write!(
            writer,
            "{}{}{}",
            champs.prefix(),
            visiteur.fields,
            champs.suffix()
        )
    }
}

#[derive(Default)]
struct ChampsEvenement {
    message: String,
    fields: String,
}

impl ChampsEvenement {
    fn add(&mut self, field: &Field, value: fmt::Arguments<'_>) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value}");
            return;
        }

        if !self.fields.is_empty() {
            self.fields.push(' ');
        }
        let _ = write!(self.fields, "{}={}", field.name(), value);
    }
}

impl Visit for ChampsEvenement {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.add(field, format_args!("{value}"));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.add(field, format_args!("{value:?}"));
    }
}

#[cfg(test)]
mod tests {
    use super::super::aide::capture;
    use super::*;

    fn render(format: PrettyFormat, emettre: impl FnOnce()) -> String {
        let fields = PrettyFormat::with_ansi(format.ansi);
        capture(format, fields, emettre)
    }

    #[test]
    fn no_colour_when_the_output_is_not_a_tty() {
        let output = render(PrettyFormat::new(), || {
            let span = tracing::info_span!("requete", request_id = "01JQ3F8K2P");
            let _input = span.enter();
            tracing::info!(status = 200, "bonjour");
        });

        assert!(
            !output.contains('\u{1b}'),
            "output colorée hors TTY : {output:?}"
        );
    }

    #[test]
    fn the_colours_are_present_when_they_are_forced() {
        let output = render(PrettyFormat::with_ansi(true), || tracing::info!("bonjour"));

        assert!(
            output.contains('\u{1b}'),
            "aucune séquence ANSI : {output:?}"
        );
    }

    #[test]
    fn the_line_carries_the_level_the_target_the_message_then_the_fields() {
        let output = render(PrettyFormat::with_ansi(false), || {
            tracing::warn!(actives = 18, max = 20, "pool proche de la saturation")
        });

        let level = output.find("WARN").expect("level absent");
        let target = output
            .find("rbs_core::logs::pretty")
            .expect("target absente");
        let message = output
            .find("pool proche de la saturation")
            .expect("message absent");
        let fields = output.find("actives=18").expect("champs absents");

        assert!(
            level < target && target < message && message < fields,
            "ordre inattendu : {output:?}"
        );
        assert!(output.contains("max=20"), "champ manquant : {output:?}");
    }

    /// La ligne colorée, figée octet à octet.
    ///
    /// C'est ce que la suppression des allocations intermédiaires ne doit pas changer :
    /// les séquences ANSI encadrent le champ *déjà* aligné, faute de quoi elles
    /// compteraient dans la largeur et décaleraient les colonnes.
    #[test]
    fn the_coloured_line_frames_each_column_around_its_padding() {
        let output = render(PrettyFormat::with_ansi(true), || tracing::info!("bonjour"));

        assert!(
            output.contains("\u{1b}[32mINFO \u{1b}[0m"),
            "le niveau doit être peint padding compris : {output:?}"
        );
        assert!(
            output.contains("\u{1b}[2mrbs_core::logs::pretty::tests\u{1b}[0m"),
            "la cible doit être peinte, sa largeur dépassée sans être tronquée : {output:?}"
        );
        assert!(
            output.contains("\u{1b}[0m  bonjour\n"),
            "le message suit la cible, séparé de deux espaces : {output:?}"
        );
    }

    /// Les champs sont écrits à la file : deux espaces avant le premier, un entre chacun.
    #[test]
    fn the_fields_are_separated_by_a_single_space_after_a_double_one() {
        let output = render(PrettyFormat::with_ansi(false), || {
            let span = tracing::info_span!("requete", request_id = "01JQ3F8K2P");
            let _input = span.enter();
            tracing::info!(status = 200, "bonjour");
        });

        assert!(
            output.contains("bonjour  status=200 request_id=01JQ3F8K2P"),
            "espacement des champs inattendu : {output:?}"
        );
    }

    #[test]
    fn the_fields_of_a_parent_span_follow_those_of_the_event() {
        let output = render(PrettyFormat::with_ansi(false), || {
            let span = tracing::info_span!("requete", request_id = "01JQ3F8K2P");
            let _input = span.enter();
            tracing::error!(status = 422, "requête refusée");
        });

        let champ_evenement = output.find("status=422").expect("champ d'événement absent");
        let champ_span = output
            .find("request_id=01JQ3F8K2P")
            .expect("champ de span absent");

        assert!(champ_evenement < champ_span, "ordre inattendu : {output:?}");
        assert!(
            !output.contains('"'),
            "valeurs entre guillemets : {output:?}"
        );
    }

    #[test]
    fn the_five_levels_render_with_their_label() {
        let output = render(PrettyFormat::with_ansi(false), || {
            tracing::trace!("t");
            tracing::debug!("d");
            tracing::info!("i");
            tracing::warn!("w");
            tracing::error!("e");
        });

        for level in ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"] {
            assert!(output.contains(level), "level {level} absent : {output:?}");
        }
    }
}
