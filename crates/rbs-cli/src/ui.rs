use std::io::{self, Write};

use console::style;

/// Une sortie que le départ de son lecteur ne fait pas paniquer.
///
/// `rbs doctor | head -3` referme le tube dès la troisième ligne : `println!` et
/// `eprintln!` paniquent sur ce `Broken pipe`, et `clap_complete` le relève par un
/// `expect`. Une commande tronquée par son lecteur rendait donc une trace de panique là
/// où trois lignes étaient tout ce qu'on lui demandait.
///
/// Un tube refermé n'est pas une faute de la commande — c'est le lecteur qui a fini de
/// lire — et la seule réponse juste est de laisser tomber ce qui restait à écrire, comme
/// le rendu du diagnostic le fait déjà ligne à ligne.
pub struct Tolerante<W: Write>(W);

impl<W: Write> Tolerante<W> {
    /// Enveloppe `sortie`.
    pub fn new(sortie: W) -> Self {
        Self(sortie)
    }
}

impl<W: Write> Write for Tolerante<W> {
    fn write(&mut self, tampon: &[u8]) -> io::Result<usize> {
        // La taille du tampon, et non zéro : `write_all` boucle tant que l'écriture
        // n'avance pas, et une sortie fermée y tournerait sans fin.
        Ok(self.0.write(tampon).unwrap_or(tampon.len()))
    }

    fn flush(&mut self) -> io::Result<()> {
        let _ = self.0.flush();
        Ok(())
    }
}

/// La sortie standard du processus, tolérante à sa fermeture.
pub fn stdout() -> Tolerante<io::Stdout> {
    Tolerante::new(io::stdout())
}

/// La sortie d'erreur du processus, tolérante à sa fermeture.
pub fn stderr() -> Tolerante<io::Stderr> {
    Tolerante::new(io::stderr())
}

/// Écrit une ligne telle quelle, sans marqueur ni retrait.
pub fn line(message: &str) {
    let _ = writeln!(stdout(), "{message}");
}

/// Seul point du CLI qui connaît `console` : la détection du TTY et le respect de
/// `NO_COLOR` sont laissés à la bibliothèque, jamais réimplémentés ailleurs.
pub fn error(message: &str) {
    let _ = writeln!(stderr(), "{} {message}", style("erreur :").red().bold());
}

/// Signale ce qui n'a pas abouti sans faire échouer la commande.
pub fn warn(message: &str) {
    let _ = writeln!(
        stderr(),
        "{} {message}",
        style("attention :").yellow().bold()
    );
}

/// Annonce ce qui a été fait.
pub fn success(message: &str) {
    line(&format!("{} {message}", style("✓").green().bold()));
}

/// Indique la suite, en retrait de ce qui précède.
pub fn info(message: &str) {
    line(&style(message).dim().to_string());
}

/// Ouvre une ligne d'attente, laissée ouverte pour que `tick` la prolonge.
///
/// La sortie est vidée à chaque écriture : sans fin de ligne, rien ne s'afficherait avant
/// que l'attente soit finie, c'est-à-dire trop tard pour renseigner qui attend.
pub fn waiting(message: &str) {
    let mut sortie = stdout();
    let _ = write!(sortie, "{} ", style(message).dim());
    let _ = sortie.flush();
}

/// Marque une seconde de plus sur la ligne ouverte par [`waiting`].
pub fn tick() {
    let mut sortie = stdout();
    let _ = write!(sortie, "{}", style(".").dim());
    let _ = sortie.flush();
}

/// Referme la ligne ouverte par [`waiting`].
pub fn end_of_line() {
    line("");
}

/// Colore un fragment en vert, pour ce qui est acquis.
pub fn green(text: &str) -> String {
    style(text).green().to_string()
}

/// Atténue un fragment, pour ce qui reste à faire.
pub fn dimmed(text: &str) -> String {
    style(text).dim().to_string()
}

/// Colore un fragment en rouge, pour ce qui est en défaut.
pub fn red(text: &str) -> String {
    style(text).red().to_string()
}

/// Colore un fragment en jaune, pour ce qui mérite un regard sans être une erreur.
pub fn yellow(text: &str) -> String {
    style(text).yellow().to_string()
}

/// Accorde un décompte de fichiers : « 1 fichier », « 3 fichiers ».
pub fn files(compte: usize) -> String {
    let pluriel = if compte > 1 { "s" } else { "" };
    format!("{compte} fichier{pluriel}")
}

/// Un puits dont toute écriture échoue, comme un tube dont le lecteur est parti.
///
/// Partagé avec les tests de `completions` : c'est la même fin de tube que
/// `clap_complete` relevait par un `expect`.
#[cfg(test)]
pub(crate) struct Rompue;

#[cfg(test)]
impl Write for Rompue {
    fn write(&mut self, _tampon: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ce que les trois puits du CLI attendent d'elle : écrire sur un tube refermé ne
    /// rend pas d'erreur, donc ne peut plus faire paniquer une macro d'affichage.
    #[test]
    fn a_broken_pipe_is_swallowed_rather_than_returned() {
        let mut sortie = Tolerante::new(Rompue);

        assert!(writeln!(sortie, "une ligne").is_ok());
        assert!(sortie.flush().is_ok());
    }

    /// `write_all` boucle tant que l'écriture n'a pas tout consommé : rendre `Ok(0)` sur
    /// une sortie fermée échangerait la panique contre une boucle sans fin.
    #[test]
    fn a_whole_buffer_is_reported_written_on_a_closed_pipe() {
        let mut sortie = Tolerante::new(Rompue);

        assert!(sortie.write_all(&[b'x'; 4096]).is_ok());
    }

    #[test]
    fn a_single_file_stays_singular() {
        assert_eq!(files(1), "1 fichier");
    }

    #[test]
    fn several_files_take_the_plural_mark() {
        assert_eq!(files(3), "3 fichiers");
    }

    /// Zéro s'écrit au singulier en français, contrairement à l'anglais.
    #[test]
    fn zero_files_stays_singular() {
        assert_eq!(files(0), "0 fichier");
    }
}
