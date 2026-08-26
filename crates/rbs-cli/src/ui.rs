use console::style;

/// Seul point du CLI qui connaît `console` : la détection du TTY et le respect de
/// `NO_COLOR` sont laissés à la bibliothèque, jamais réimplémentés ailleurs.
pub fn error(message: &str) {
    eprintln!("{} {message}", style("erreur :").red().bold());
}
