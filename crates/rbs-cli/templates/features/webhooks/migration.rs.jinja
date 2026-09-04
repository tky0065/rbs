use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WebhookSubscriptions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WebhookSubscriptions::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WebhookSubscriptions::Url)
                            .string()
                            .not_null(),
                    )
                    // Les motifs écoutés, tableau de chaînes. Le tri se fait en Rust : la
                    // recherche dans un tableau JSON n'a pas de forme commune aux trois
                    // moteurs, et les motifs à préfixe l'auraient de toute façon interdite.
                    .col(
                        ColumnDef::new(WebhookSubscriptions::Events)
                            .json()
                            .not_null(),
                    )
                    // En clair, et il ne peut pas ne pas l'être : la livraison le relit pour
                    // signer, là où un mot de passe n'a jamais à être relu. La table est
                    // donc aussi sensible que les données qu'elle protège.
                    .col(
                        ColumnDef::new(WebhookSubscriptions::Secret)
                            .string()
                            .not_null(),
                    )
                    // Une date plutôt qu'un booléen : elle porte le booléen et le moment.
                    // La ligne survit à sa révocation, et une livraison déjà en file y
                    // trouve encore de quoi savoir qu'elle n'a plus lieu d'être.
                    .col(
                        ColumnDef::new(WebhookSubscriptions::RevokedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(WebhookSubscriptions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(WebhookSubscriptions::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await

        // Aucun index de plus, et ce n'est pas un oubli : l'émission lit tous les
        // abonnements non révoqués pour les trier elle-même, et un index sur une colonne
        // nulle dans presque toutes les lignes n'épargnerait pas ce parcours. La table
        // compte les abonnés du projet, non les événements émis.
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WebhookSubscriptions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum WebhookSubscriptions {
    Table,
    Id,
    Url,
    Events,
    Secret,
    RevokedAt,
    CreatedAt,
    UpdatedAt,
}
