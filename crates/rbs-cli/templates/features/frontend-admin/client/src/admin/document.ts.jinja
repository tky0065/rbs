/**
 * Ce que le document OpenAPI dit du service, et que nulle route ne dit.
 *
 * La version et le nombre de routes ne sont exposés par aucune opération du contrat : ce
 * sont des propriétés du document lui-même, et le client engendré ne peut pas les porter
 * puisqu'il en sort. C'est la seule requête du shell qui ne passe pas par lui — et elle
 * ne vise aucune route du contrat, ce qui est exactement pourquoi elle vit ici et non
 * dans un écran.
 */

const DOCUMENT = '/api-docs/openapi.json'

/**
 * Les seules clés d'un chemin qui sont des opérations : à côté d'elles, il peut porter
 * `parameters`, `summary` ou `$ref`, qui n'en sont pas et gonfleraient le compte.
 */
const METHODES = ['get', 'post', 'put', 'patch', 'delete', 'head', 'options', 'trace']

type Publie = {
  openapi?: string
  info?: { version?: string }
  paths?: Record<string, Record<string, unknown>>
}

/** Ce que le contrat publie de lui-même. */
export interface Contrat {
  version: string | null
  routes: number
}

/**
 * Lit le document publié, ou jette.
 *
 * Un 200 ne prouve rien : le binaire rend l'application pour toute route qu'il ne connaît
 * pas, si bien qu'un document coupé sous `[docs]` ferait répondre le client à sa place.
 * C'est donc la présence de `openapi` — le seul champ que la spécification impose — qui
 * atteste le document, et rien d'autre.
 */
export async function lireLeContrat(): Promise<Contrat> {
  const reponse = await fetch(DOCUMENT, { headers: { accept: 'application/json' } })

  if (!reponse.ok) {
    throw new Error(DOCUMENT)
  }

  const publie = (await reponse.json()) as Publie

  if (typeof publie.openapi !== 'string') {
    throw new Error(DOCUMENT)
  }

  return {
    version: publie.info?.version ?? null,
    routes: Object.values(publie.paths ?? {}).reduce(
      (total, operations) =>
        total + Object.keys(operations).filter((clef) => METHODES.includes(clef)).length,
      0,
    ),
  }
}
