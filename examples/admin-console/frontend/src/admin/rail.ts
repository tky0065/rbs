import type { Component } from 'vue'

import { GaugeIcon, KeyRoundIcon, UserRoundIcon } from '@lucide/vue'

import { TEXTES } from './textes'

/** Une entrée du rail : ce qu'elle ouvre, ce qu'elle dit, et son affordance. */
export interface EntreeDuRail {
  /** Le nom de la route, tel que `montage.ts` la déclare. */
  route: string
  libelle: string
  /** Rendue devant le libellé, quand l'entrée en porte une. */
  icone?: Component
}

/**
 * Les entrées du rail, dans l'ordre où elles s'affichent.
 *
 * Une seule liste pour les deux rails que le shell rend — celui à demeure et celui du
 * panneau latéral. Portées à la main dans les deux, les entrées divergeaient à la
 * première retouche : un écran ajouté d'un seul côté restait invisible sur téléphone,
 * sans que rien n'échoue.
 *
 * C'est aussi ce qui permet à un écran engendré de s'y poser par une ancre : le mécanisme
 * ne connaît que les commentaires `//` et `#`, et n'aurait su viser aucune ligne d'un
 * `<template>`.
 */
export const RAIL: EntreeDuRail[] = [
  { route: 'admin-tableau', libelle: TEXTES.tableau, icone: GaugeIcon },
  { route: 'admin-sessions', libelle: TEXTES.sessions, icone: KeyRoundIcon },
  { route: 'admin-profil', libelle: TEXTES.profil, icone: UserRoundIcon },
  // <rbs:admin_rail>
  { route: 'admin-demonstration', libelle: 'Démonstration' },
  { route: 'admin-incidents', libelle: 'Incidents' },
  // </rbs:admin_rail>
]
