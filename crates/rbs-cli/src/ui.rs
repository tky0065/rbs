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
