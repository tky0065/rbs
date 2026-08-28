use std::fmt;

use serde_json::{Map, Value};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::field::RecordFields;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::{ChronoUtc, FormatTime};
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields, FormattedFields};
use tracing_subscriber::registry::LookupSpan;

const HORODATAGE: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

/// Formateur d'événements `tracing` destiné à l'exploitation.
///
/// Rend un objet JSON par ligne, portant `ts`, `level`, `target`, `msg`, puis les champs
/// de l'événement et de ses spans parents, à plat et dans leur type d'origine.
pub struct JsonFormat {
    horodatage: ChronoUtc,
}

impl JsonFormat {
    /// Construit le formateur.
    pub fn new() -> Self {
        Self {
            horodatage: ChronoUtc::new(HORODATAGE.to_owned()),
        }
    }

    fn timestamp(&self) -> String {
        let mut rendered = String::new();
        // `FormatTime` n'écrit que dans un `Writer` : le détour par une chaîne est le
        // seul moyen d'obtenir la valeur à insérer dans l'objet JSON.
        let _ = self.horodatage.format_time(&mut Writer::new(&mut rendered));
        rendered
    }
}

impl Default for JsonFormat {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, N> FormatEvent<S, N> for JsonFormat
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

        let mut visiteur = ChampsJson::default();
        event.record(&mut visiteur);

        let mut objet = visiteur.fields;
        objet.insert("ts".to_owned(), Value::String(self.timestamp()));
        objet.insert(
            "level".to_owned(),
            Value::String(metadata.level().to_string()),
        );
        objet.insert(
            "target".to_owned(),
            Value::String(metadata.target().to_owned()),
        );
        objet.insert("msg".to_owned(), Value::String(visiteur.message));

        // Le registry ne conserve les champs d'un span que sous forme de texte déjà
        // formaté. Les relire est le seul accès à leur contenu sans écrire une `Layer`
        // maison ; l'implémentation de `FormatFields` ci-dessous les y écrit en JSON.
        if let Some(portee) = ctx.event_scope() {
            for span in portee {
                let extensions = span.extensions();
                let Some(formates) = extensions.get::<FormattedFields<N>>() else {
                    continue;
                };
                let Ok(Value::Object(fields)) = serde_json::from_str(&formates.fields) else {
                    continue;
                };
                for (cle, value) in fields {
                    objet.entry(cle).or_insert(value);
                }
            }
        }

        writeln!(writer, "{}", Value::Object(objet))
    }
}

impl<'writer> FormatFields<'writer> for JsonFormat {
    fn format_fields<R: RecordFields>(
        &self,
        mut writer: Writer<'writer>,
        fields: R,
    ) -> fmt::Result {
        let mut visiteur = ChampsJson::default();
        fields.record(&mut visiteur);

        if !visiteur.message.is_empty() {
            visiteur
                .fields
                .insert("msg".to_owned(), Value::String(visiteur.message));
        }
        write!(writer, "{}", Value::Object(visiteur.fields))
    }
}

#[derive(Default)]
struct ChampsJson {
    message: String,
    fields: Map<String, Value>,
}

impl ChampsJson {
    fn insert(&mut self, field: &Field, value: Value) {
        self.fields.insert(field.name().to_owned(), value);
    }
}

impl Visit for ChampsJson {
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.insert(field, Value::Bool(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.insert(field, Value::from(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.insert(field, Value::from(value));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.insert(field, Value::from(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_owned();
            return;
        }
        self.insert(field, Value::String(value.to_owned()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let rendered = format!("{value:?}");
        if field.name() == "message" {
            self.message = rendered;
            return;
        }
        self.insert(field, Value::String(rendered));
    }
}

#[cfg(test)]
mod tests {
    use super::super::aide::capture;
    use super::*;

    fn render(emettre: impl FnOnce()) -> String {
        capture(JsonFormat::new(), JsonFormat::new(), emettre)
    }

    #[test]
    fn each_line_is_valid_json_carrying_ts_level_and_msg() {
        let output = render(|| {
            tracing::info!("serveur démarré");
            tracing::warn!(actives = 18, "pool proche de la saturation");
            tracing::error!("requête refusée");
        });

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3, "trois lines attendues : {output:?}");
        for line in lines {
            let objet: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("line non JSON ({e}) : {line}"));
            for cle in ["ts", "level", "msg"] {
                assert!(objet.get(cle).is_some(), "clé {cle} absente : {line}");
            }
        }
    }

    #[test]
    fn the_fields_keep_their_json_type() {
        let output =
            render(|| tracing::error!(status = 422, latency_ms = 12.4, actif = true, "refus"));

        let objet: serde_json::Value = serde_json::from_str(output.trim()).expect("line non JSON");
        assert_eq!(objet["status"], serde_json::json!(422));
        assert_eq!(objet["latency_ms"], serde_json::json!(12.4));
        assert_eq!(objet["actif"], serde_json::json!(true));
        assert_eq!(objet["msg"], serde_json::json!("refus"));
    }

    #[test]
    fn the_fields_of_a_parent_span_surface_in_the_object() {
        let output = render(|| {
            let span = tracing::info_span!("requete", request_id = "01JQ3F8K2P");
            let _input = span.enter();
            tracing::error!(status = 422, "requête refusée");
        });

        let objet: serde_json::Value = serde_json::from_str(output.trim()).expect("line non JSON");
        assert_eq!(objet["request_id"], serde_json::json!("01JQ3F8K2P"));
        assert_eq!(objet["status"], serde_json::json!(422));
    }
}
