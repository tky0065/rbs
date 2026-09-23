/** Une ligne référencée, telle qu'un écran la montre : son identifiant et son libellé. */
export interface Entree {
  cle: string
  libelle: string
}

/** L'identifiant raccourci, quand aucun libellé ne le remplace. */
export function courte(cle: string): string {
  return cle.length > 8 ? `${cle.slice(0, 8)}…` : cle
}

/**
 * Les libellés des identifiants d'une page, en un appel.
 *
 * Ne garde que les identifiants demandés : une source qui ignorerait l'opérateur `in` rend
 * des lignes quelconques, et un libellé attribué à la mauvaise ligne serait pire qu'un
 * identifiant. Une panne rend une table vide — l'écran montre alors les identifiants.
 */
export async function resoudre(
  lire: (ids: string[]) => Promise<Entree[]>,
  ids: readonly (string | null)[],
): Promise<Map<string, string>> {
  const demandes = [...new Set(ids.filter((id): id is string => typeof id === 'string'))]
  const libelles = new Map<string, string>()

  if (demandes.length === 0) {
    return libelles
  }

  try {
    const voulus = new Set(demandes)
    for (const entree of await lire(demandes)) {
      if (voulus.has(entree.cle)) {
        libelles.set(entree.cle, entree.libelle)
      }
    }
  } catch {
    // Un refus (403 pour un compte qui n'est pas admin) ou une panne : la table reste
    // lisible par ses identifiants, et aucune alerte ne couvre l'écran.
  }

  return libelles
}

/** La panne est-elle un refus de droits ? */
export function refusee(cause: unknown): boolean {
  return typeof cause === 'object' && cause !== null && 'status' in cause && cause.status === 403
}
