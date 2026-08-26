//! `include_dir!` embarque les templates sans que Cargo sache d'où elles viennent : sans
//! cette déclaration, un binaire déjà compilé garde l'arborescence de la compilation
//! précédente, et une template corrigée n'atteint jamais le projet généré.
fn main() {
    println!("cargo::rerun-if-changed=../../templates");
}
