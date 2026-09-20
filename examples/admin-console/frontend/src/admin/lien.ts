import type { RouteLocationNormalizedLoaded } from 'vue-router'

/**
 * Le jeton que porte le lien d'un courriel, ou la chaîne vide.
 *
 * Le fragment d'abord, parce que c'est là que le fragment `auth` le met —
 * `…/reset-password#token=…`. Un navigateur ne l'envoie jamais au serveur : ni les
 * journaux d'accès du client, ni l'en-tête `Referer` d'une page qu'il charge ensuite ne
 * portent alors le jeton. La query est relue à défaut, pour le lien qu'un client de
 * messagerie a réécrit ou qu'un opérateur recolle à la main.
 *
 * Un module à part, et non une fonction de l'un des deux écrans : la réinitialisation et
 * la vérification lisent le même lien, et deux copies de cette lecture divergeraient le
 * jour où `auth` changerait de place.
 */
export function jetonDuLien(route: RouteLocationNormalizedLoaded): string {
  const fragment = new URLSearchParams(route.hash.replace(/^#/, ''))
  const porte = fragment.get('token') ?? route.query.token

  return typeof porte === 'string' ? porte : ''
}
