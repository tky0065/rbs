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

    fn horodater(&self) -> String {
        let mut rendu = String::new();
        // `FormatTime` n'écrit que dans un `Writer` : le détour par une chaîne est le
        // seul moyen d'obtenir la valeur à insérer dans l'objet JSON.
        let _ = self.horodatage.format_time(&mut Writer::new(&mut rendu));
        rendu
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
        let metadonnees = event.metadata();

        let mut visiteur = ChampsJson::default();
        event.record(&mut visiteur);

        let mut objet = visiteur.champs;
        objet.insert("ts".to_owned(), Value::String(self.horodater()));
        objet.insert(
            "level".to_owned(),
            Value::String(metadonnees.level().to_string()),
        );
        objet.insert(
            "target".to_owned(),
            Value::String(metadonnees.target().to_owned()),
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
                let Ok(Value::Object(champs)) = serde_json::from_str(&formates.fields) else {
                    continue;
                };
                for (cle, valeur) in champs {
                    objet.entry(cle).or_insert(valeur);
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
                .champs
                .insert("msg".to_owned(), Value::String(visiteur.message));
        }
        write!(writer, "{}", Value::Object(visiteur.champs))
    }
}

#[derive(Default)]
struct ChampsJson {
    message: String,
    champs: Map<String, Value>,
}

impl ChampsJson {
    fn inserer(&mut self, champ: &Field, valeur: Value) {
        self.champs.insert(champ.name().to_owned(), valeur);
    }
}

impl Visit for ChampsJson {
    fn record_bool(&mut self, champ: &Field, valeur: bool) {
        self.inserer(champ, Value::Bool(valeur));
    }

    fn record_i64(&mut self, champ: &Field, valeur: i64) {
        self.inserer(champ, Value::from(valeur));
    }

    fn record_u64(&mut self, champ: &Field, valeur: u64) {
        self.inserer(champ, Value::from(valeur));
    }

    fn record_f64(&mut self, champ: &Field, valeur: f64) {
        self.inserer(champ, Value::from(valeur));
    }

    fn record_str(&mut self, champ: &Field, valeur: &str) {
        if champ.name() == "message" {
            self.message = valeur.to_owned();
            return;
        }
        self.inserer(champ, Value::String(valeur.to_owned()));
    }

    fn record_debug(&mut self, champ: &Field, valeur: &dyn fmt::Debug) {
        let rendu = format!("{valeur:?}");
        if champ.name() == "message" {
            self.message = rendu;
            return;
        }
        self.inserer(champ, Value::String(rendu));
    }
}

#[cfg(test)]
mod tests {
    use super::super::aide::capture;
    use super::*;

    fn rendre(emettre: impl FnOnce()) -> String {
        capture(JsonFormat::new(), JsonFormat::new(), emettre)
    }

    #[test]
    fn chaque_ligne_est_un_json_valide_portant_ts_level_et_msg() {
        let sortie = rendre(|| {
            tracing::info!("serveur démarré");
            tracing::warn!(actives = 18, "pool proche de la saturation");
            tracing::error!("requête refusée");
        });

        let lignes: Vec<&str> = sortie.lines().collect();
        assert_eq!(lignes.len(), 3, "trois lignes attendues : {sortie:?}");
        for ligne in lignes {
            let objet: serde_json::Value = serde_json::from_str(ligne)
                .unwrap_or_else(|e| panic!("ligne non JSON ({e}) : {ligne}"));
            for cle in ["ts", "level", "msg"] {
                assert!(objet.get(cle).is_some(), "clé {cle} absente : {ligne}");
            }
        }
    }

    #[test]
    fn les_champs_conservent_leur_type_json() {
        let sortie =
            rendre(|| tracing::error!(status = 422, latency_ms = 12.4, actif = true, "refus"));

        let objet: serde_json::Value = serde_json::from_str(sortie.trim()).expect("ligne non JSON");
        assert_eq!(objet["status"], serde_json::json!(422));
        assert_eq!(objet["latency_ms"], serde_json::json!(12.4));
        assert_eq!(objet["actif"], serde_json::json!(true));
        assert_eq!(objet["msg"], serde_json::json!("refus"));
    }

    #[test]
    fn les_champs_d_un_span_parent_remontent_dans_l_objet() {
        let sortie = rendre(|| {
            let span = tracing::info_span!("requete", request_id = "01JQ3F8K2P");
            let _entree = span.enter();
            tracing::error!(status = 422, "requête refusée");
        });

        let objet: serde_json::Value = serde_json::from_str(sortie.trim()).expect("ligne non JSON");
        assert_eq!(objet["request_id"], serde_json::json!("01JQ3F8K2P"));
        assert_eq!(objet["status"], serde_json::json!(422));
    }
}
