import type { Montage } from '@/router'

import { garde } from './garde'

/**
 * Ce que l'espace d'administration monte dans le routeur du socle.
 *
 * Quatre routes publiques — tout ce qu'un visiteur doit atteindre sans jeton pour obtenir
 * le sien — et tout le reste derrière la garde. Elles vivent hors du shell, qui n'a ni
 * rail ni compte à montrer à qui n'est pas encore entré. Chaque écran part dans son propre
 * morceau : qui ne visite que l'accueil ne télécharge jamais l'administration.
 *
 * Les alias sont les chemins que le fragment `auth` met dans ses courriels. Il les compose
 * depuis `app_url` sans rien savoir d'un shell posé à côté : c'est donc au shell de les
 * servir, et un alias les sert sans redirection — le jeton du lien vit dans le fragment de
 * l'URL, qu'une redirection perdrait.
 */
export const routes: Montage['routes'] = [
  {
    path: '/admin/connexion',
    name: 'admin-connexion',
    alias: '/forgot-password',
    component: () => import('./vues/Connexion.vue'),
  },
  {
    path: '/admin/inscription',
    name: 'admin-inscription',
    component: () => import('./vues/Inscription.vue'),
  },
  {
    path: '/admin/reinitialisation',
    name: 'admin-reinitialisation',
    alias: '/reset-password',
    component: () => import('./vues/Reinitialisation.vue'),
  },
  {
    path: '/admin/verification',
    name: 'admin-verification',
    alias: '/verify-email',
    component: () => import('./vues/Verification.vue'),
  },
  // region: montage
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
  // endregion: montage
]

export { garde }
