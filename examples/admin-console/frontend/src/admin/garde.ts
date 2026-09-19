import type { NavigationGuardWithThis } from 'vue-router'

/** Le préfixe de tout ce que cette garde protège. */
const ESPACE = '/admin'

/**
 * Renvoie à la connexion toute route d'administration atteinte sans session.
 *
 * L'attente est ici, et non dans l'écran visé : restaurer la session pendant que l'écran
 * se monte le montrerait vide, puis rempli, puis remplacé par la connexion. La garde ne
 * rend la main qu'une fois la question tranchée, et le navigateur n'affiche rien entre
 * les deux.
 */
export const garde: NavigationGuardWithThis<undefined> = async (vers) => {
  if (!vers.path.startsWith(ESPACE) || vers.name === 'admin-connexion') {
    return true
  }

  // Importé ici et non en tête de fichier : le routeur du socle lit ce module dès le
  // premier écran, et une importation statique tirerait la couche d'état et tout le
  // client engendré dans le morceau d'entrée — que télécharge le visiteur de l'accueil,
  // qui n'ira jamais dans l'administration.
  const { useAuthentification } = await import('@/stores/authentification')
  const authentification = useAuthentification()

  if (await authentification.restaurer()) {
    return true
  }

  // La route visée voyage avec la redirection : l'opérateur revient là où il allait, et
  // non sur une page qu'il n'a pas demandée.
  return { name: 'admin-connexion', query: { suite: vers.fullPath } }
}
