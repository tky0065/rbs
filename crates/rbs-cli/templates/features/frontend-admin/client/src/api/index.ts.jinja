/**
 * Le client de l'API, et le seul de l'application.
 *
 * `./client` est engendré par `rbs generate client --lang ts --out frontend/src/api`
 * depuis le document OpenAPI du projet : ses chemins, ses corps et ses réponses sont ceux
 * du contrat, vérifiés à la compilation. Le régénérer après chaque changement de contrat
 * est le seul entretien — rien ici ne réécrit de couche HTTP par-dessus lui, et un écran
 * qui appellerait `fetch` de lui-même perdrait cette vérification.
 */
import { ApiClient, ApiError } from './client'
import { jetonAcces } from './jetons'

// L'application est servie par le binaire qui sert l'API : même origine, donc racine
// vide. En développement, c'est le relais déclaré dans `vite.config.ts` qui l'atteint.
export const api = new ApiClient({
  baseUrl: '',
  // Une fonction, et non une carte figée : le client la rappelle à chaque requête, et le
  // jeton d'accès change à chaque renouvellement.
  headers: (): Record<string, string> => {
    const jeton = jetonAcces()

    return jeton === null ? {} : { authorization: `Bearer ${jeton}` }
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
