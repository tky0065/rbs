/**
 * Le client de l'API, et le seul de l'application.
 *
 * `./client` est le **client engendré** par `rbs generate client --lang ts` depuis le
 * **contrat** du projet : ses chemins, ses corps et ses réponses sont ceux du contrat,
 * vérifiés à la compilation. Le régénérer après chaque changement de contrat est le seul
 * entretien — rien ici ne réécrit de couche HTTP par-dessus lui, et un écran qui
 * appellerait `fetch` de lui-même perdrait cette vérification.
 *
 * Ce module appartient au socle et non au shell d'administration : la couche de transport
 * est le prérequis de l'application, pas son complément. Un projet qui n'a posé que le
 * socle parle donc déjà à son API.
 */
import { ApiClient, ApiError } from './client'

/**
 * Ce qu'un fragment ajoute aux en-têtes de chaque requête.
 *
 * Même mécanisme que le montage du routeur, et pour la même raison : le registre des
 * ancres est clos, et un fragment ne peut pas redéposer un fichier qu'un autre a posé.
 * Tout module nommé `entetes.ts` sous `src/` est lu par Vite à la construction, et ce
 * qu'il exporte s'ajoute ici. Aucun fragment posé, aucun module trouvé — et le socle
 * appelle son API sans en-tête, ce qui suffit à toute route ouverte.
 */
export interface Entetes {
  entetes: () => Record<string, string>
}

const fournisseurs = Object.values(import.meta.glob<Entetes>('../**/entetes.ts', { eager: true }))

// L'application est servie par le binaire qui sert l'API : même origine, donc racine
// vide. En développement, c'est le relais déclaré dans `vite.config.ts` qui l'atteint.
export const api = new ApiClient({
  baseUrl: '',
  // Une fonction, et non une carte figée : le client la rappelle à chaque requête, et le
  // jeton d'accès que pose le shell d'administration change à chaque renouvellement.
  headers: (): Record<string, string> => {
    const entetes: Record<string, string> = {}

    for (const fournisseur of fournisseurs) {
      Object.assign(entetes, fournisseur.entetes())
    }

    return entetes
  },
})

/**
 * La phrase que porte une panne d'API, ou `defaut` quand elle n'en porte pas.
 *
 * Le noyau rend ses erreurs en RFC 9457 : `detail` dit ce qui s'est passé pour cette
 * requête-ci, `title` ne dit que la classe de la panne. Un écran affiche donc le premier
 * quand il existe, et jamais un code brut — que l'opérateur ne saurait pas lire.
 */
export function phrase(faute: unknown, defaut: string): string {
  if (faute instanceof ApiError) {
    return faute.problem?.detail ?? faute.problem?.title ?? defaut
  }

  return defaut
}

export { ApiError }
