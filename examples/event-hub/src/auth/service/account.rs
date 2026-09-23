use rbs_core::{Error, Page, Pagination, Result};
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;

use super::super::config::FlowConfig;
use super::super::dto::{EmailRequest, UserFilter, UserSummary};
use super::super::repository::{self, Model};
use super::{normalise, verification, warn_taken};
use crate::modules::mail::Mailer;

/// Change l'adresse d'un compte, sans que la réponse dise si la nouvelle était prise.
///
/// Libre, elle est écrite et la preuve qui portait sur l'ancienne tombe : le lien de
/// vérification part détaché vers la nouvelle. Prise, rien n'est touché et son titulaire
/// est prévenu de la tentative, en détaché aussi. L'appelant reçoit la même chose dans les
/// deux cas — c'est la règle de `register`, et elle vaut ici d'autant plus qu'un compte
/// suffirait sinon à savoir quelles adresses le service connaît.
pub async fn change_email(
    db: &DatabaseConnection,
    mail: &Mailer,
    flows: &FlowConfig,
    id: Uuid,
    input: EmailRequest,
) -> Result<()> {
    let email = normalise(&input.email);

    // Un jeton valide dont le compte a disparu ne vaut pas mieux qu'un jeton invalide :
    // la même lecture, et la même erreur, qu'à `me`.
    let compte = repository::find(db, id).await?.ok_or(Error::Unauthorized)?;

    // La sienne : rien à écrire, et rien à prouver. Repasser par l'écriture retirerait une
    // preuve déjà acquise pour la redemander aussitôt.
    if compte.email == email {
        return Ok(());
    }

    // L'écriture d'abord, la lecture seulement si elle échoue : c'est la contrainte
    // d'unicité qui tranche — une lecture préalable laisserait passer deux changements
    // concurrents vers la même adresse — et les deux branches coûtent alors une requête
    // chacune, que le temps de réponse ne distingue pas.
    if repository::user::set_email(db, compte.id, &email).await? {
        // Le compte tel qu'il vient d'être écrit, et non tel qu'il a été lu : c'est à la
        // nouvelle adresse que le lien part, et c'est elle qu'il prouvera.
        verification::send_link_detached(
            db,
            mail,
            flows,
            Model {
                email,
                email_verified_at: None,
                ..compte
            },
        );

        return Ok(());
    }

    if let Some(titulaire) = repository::find_by_email(db, &email).await? {
        warn_taken(mail, flows, titulaire);
    }

    Ok(())
}

/// Une page de comptes, réduits à ce qu'une liste de sélection montre.
pub async fn filter_users(
    db: &DatabaseConnection,
    mut filtre: UserFilter,
    pagination: &Pagination,
) -> Result<Page<UserSummary>> {
    // Les adresses sont écrites en minuscules : `LIKE` et `=` distinguent la casse sous
    // PostgreSQL, et une recherche tapée en majuscules n'y trouverait rien.
    if let Some(email) = filtre.email.as_mut() {
        email.eq = email.eq.as_deref().map(str::to_lowercase);
        email.contains = email.contains.as_deref().map(str::to_lowercase);
    }

    let (comptes, total) = repository::filter(db, &filtre, pagination).await?;

    Ok(Page::new(
        comptes
            .into_iter()
            .map(|compte| UserSummary {
                id: compte.id,
                email: compte.email,
            })
            .collect(),
        pagination,
        total,
    ))
}
