import type { Montage } from '@/router'

import { garde } from './garde'

/**
 * Ce que l'espace d'administration monte dans le routeur du socle.
 *
 * Une seule route publique — la connexion — et tout le reste derrière la garde. Chaque
 * écran part dans son propre morceau : qui ne visite que l'accueil ne télécharge jamais
 * l'administration.
 */
export const routes: Montage['routes'] = [
  {
    path: '/admin/connexion',
    name: 'admin-connexion',
    component: () => import('./vues/Connexion.vue'),
  },
  // Sans nom : le tableau de bord occupe le chemin vide, et un nom porté par le parent
  // désignerait une route que le routeur ne saurait pas rendre seule.
  {
    path: '/admin',
    component: () => import('./Shell.vue'),
    children: [
      {
        path: '',
        name: 'admin-tableau',
        component: () => import('./vues/TableauDeBord.vue'),
      },
      {
        path: 'sessions',
        name: 'admin-sessions',
        component: () => import('./vues/Sessions.vue'),
      },
      {
        path: 'profil',
        name: 'admin-profil',
        component: () => import('./vues/Profil.vue'),
      },
      // <rbs:admin_routes>
      {
        path: 'demonstration',
        name: 'admin-demonstration',
        component: () => import('./vues/Demonstration.vue'),
      },
      {
        path: 'incidents',
        name: 'admin-incidents',
        component: () => import('./vues/Incidents.vue'),
      },
      // </rbs:admin_routes>
    ],
  },
]

export { garde }
