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

    fn paint(&self, style: Style, text: &str) -> String {
        if self.ansi {
            style.paint(text).to_string()
        } else {
            text.to_owned()
        }
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

        // La couleur est appliquée après l'alignement : les séquences ANSI comptent
        // dans la largeur demandée à `format!` et décaleraient les colonnes.
        let level = format!("{:<LARGEUR_NIVEAU$}", metadata.level().as_str());
        let style = Self::level_style(metadata.level());
        write!(writer, "  {}", self.paint(style, &level))?;

        let target = format!("{:<LARGEUR_CIBLE$}", metadata.target());
        write!(writer, "  {}", self.paint(Style::new().dimmed(), &target))?;

        let mut visiteur = ChampsEvenement::default();
        event.record(&mut visiteur);
        write!(writer, "  {}", visiteur.message)?;

        // Les champs des spans parents suivent ceux de l'événement : sans eux, le
        // `request_id` que le middleware attache au span ne serait jamais journalisé.
        // Ils arrivent déjà peints par l'implémentation de `FormatFields` ci-dessous.
        let mut fields = Vec::new();
        if !visiteur.fields.is_empty() {
            fields.push(self.paint(Style::new().dimmed(), &visiteur.fields));
        }
        if let Some(portee) = ctx.event_scope() {
            for span in portee {
                let extensions = span.extensions();
                let Some(formates) = extensions.get::<FormattedFields<N>>() else {
                    continue;
                };
                if !formates.fields.is_empty() {
                    fields.push(formates.fields.clone());
                }
            }
        }

        if !fields.is_empty() {
            write!(writer, "  {}", fields.join(" "))?;
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
        write!(
            writer,
            "{}",
            self.paint(Style::new().dimmed(), &visiteur.fields)
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
        let fields = output.find("actives=18").expect("fields absents");

        assert!(
            level < target && target < message && message < fields,
            "ordre inattendu : {output:?}"
        );
        assert!(output.contains("max=20"), "field manquant : {output:?}");
    }

    #[test]
    fn the_fields_of_a_parent_span_follow_those_of_the_event() {
        let output = render(PrettyFormat::with_ansi(false), || {
            let span = tracing::info_span!("requete", request_id = "01JQ3F8K2P");
            let _input = span.enter();
            tracing::error!(status = 422, "requête refusée");
        });

        let champ_evenement = output.find("status=422").expect("field d'événement absent");
        let champ_span = output
            .find("request_id=01JQ3F8K2P")
            .expect("field de span absent");

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
