//! Montage partagé par les tests des deux formateurs.

use std::io;
use std::sync::{Arc, Mutex};

use tracing_subscriber::Registry;
use tracing_subscriber::fmt::{FormatEvent, FormatFields, MakeWriter};

#[derive(Clone, Default)]
pub(super) struct Tampon(Arc<Mutex<Vec<u8>>>);

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

/// Émet des événements dans un abonné jetable et rend ce qui a été écrit.
pub(super) fn capture<E, F>(evenement: E, champs: F, emettre: impl FnOnce()) -> String
where
    E: FormatEvent<Registry, F> + Send + Sync + 'static,
    F: for<'a> FormatFields<'a> + Send + Sync + 'static,
{
    let tampon = Tampon::default();
    let abonne = tracing_subscriber::fmt()
        .fmt_fields(champs)
        .event_format(evenement)
        .with_writer(tampon.clone())
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::with_default(abonne, emettre);
    tampon.contenu()
}
