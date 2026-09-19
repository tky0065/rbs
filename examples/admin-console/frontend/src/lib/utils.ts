import type { ClassValue } from 'clsx'

import { clsx } from 'clsx'
import { twMerge } from 'tailwind-merge'

/**
 * Compose des classes utilitaires, la dernière l'emportant sur celles qu'elle contredit.
 *
 * Sans la fusion, `class="p-0"` passé à un composant qui écrit déjà `p-2` laisserait les
 * deux dans l'attribut, et c'est l'ordre de la feuille — non celui de l'appel — qui
 * trancherait. Tout composant du projet passe donc sa classe par ici.
 */
export function cn(...classes: ClassValue[]) {
  return twMerge(clsx(classes))
}
