/** Les libellés de l'accueil, rendus dans la langue du projet à sa création. */
export const TEXTES = {
  titre: "L'API tourne, et cette page en vient",
  sous_titre:
    "Rien de ce qui suit n'est écrit d'avance : la page interroge le service qui la sert, et imprime ce qui revient.",
  sonde: 'Sonde de santé',
  journal_vide: 'première lecture en route…',
  reimprimer: 'Réimprimer',
  cadence: "Relue toutes les quinze secondes. Chaque lecture s'imprime sous la précédente.",
  injoignable: 'injoignable',
  essai: "Essayer l'API",
  essai_note: "Telle quelle, contre l'adresse qui a servi cette page.",
  copier: 'Copier',
  copie: 'Copiée',
  routes: 'Routes documentées',
  routes_source: 'Lues dans le document OpenAPI que ce service publie.',
  routes_vides: {
    attente: 'lecture du document…',
    servi: 'le document est servi, et ne déclare aucune route.',
    coupe: "le document n'est pas servi : `openapi_json` sous `[docs]`, dans `config/default.toml`.",
    injoignable: "le service n'a pas répondu : rien ne peut être dit de son document.",
  },
  methode: 'Méthode',
  chemin: 'Route',
  operation: 'Opération',
  route_une: 'route',
  routes_plusieurs: 'routes',
  documentation: 'Documentation',
  interface_docs: 'Interface',
  document_docs: 'Document',
  non_servi: 'non servi',
  swagger_absent: '`swagger_ui` est coupé sous `[docs]`.',
  document_absent: '`openapi_json` est coupé sous `[docs]`.',
  docs_muet: "le service n'a pas répondu : rien ne peut être dit de `[docs]`.",
  attente: 'interrogation…',
  composants: "Composants d'interface",
  galerie: 'Voir la galerie',
  galerie_note:
    'Les quatorze composants que le socle a déposés dans le projet, chacun dans ses états principaux.',
  pied_fichier:
    'Cette page est `frontend/src/views/Accueil.vue`, ses libellés `frontend/src/views/accueil-textes.ts`.',
  pied_bandes: "Chaque bande ci-dessus est une `Bande` : en remplacer une n'en touche aucune autre.",
}

