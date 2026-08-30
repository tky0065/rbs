use console::style;

/// Seul point du CLI qui connaît `console` : la détection du TTY et le respect de
/// `NO_COLOR` sont laissés à la bibliothèque, jamais réimplémentés ailleurs.
pub fn error(message: &str) {
    eprintln!("{} {message}", style("erreur :").red().bold());
}

/// Signale ce qui n'a pas abouti sans faire échouer la commande.
pub fn warn(message: &str) {
    eprintln!("{} {message}", style("attention :").yellow().bold());
}

/// Annonce ce qui a été fait.
pub fn success(message: &str) {
    println!("{} {message}", style("✓").green().bold());
}

/// Indique la suite, en retrait de ce qui précède.
pub fn info(message: &str) {
    println!("{}", style(message).dim());
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

#[cfg(test)]
mod tests {
    use super::*;

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
