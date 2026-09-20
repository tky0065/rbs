import { defineStore } from 'pinia'
import { ref } from 'vue'
import { toast } from 'vue-sonner'

import type { Theme } from '@/lib/theme'

import { appliquerTheme, retenirTheme, themeInitial } from '@/lib/theme'

/**
 * L'état de l'interface : son thème, son rail, et ce qu'elle dit à l'opérateur.
 *
 * Le second et dernier store de l'application. Ce qui vit ici n'appartient à aucun écran
 * en particulier et survit à leur navigation — le reste est de l'état de composant.
 */
export const useInterface = defineStore('interface', () => {
  // Le thème est au socle, qui l'a déjà posé avant le montage : le store n'en est que
  // l'interrupteur, et le relire ici évite qu'un écran d'administration reparte du clair
  // sur une machine réglée en sombre.
  const theme = ref<Theme>(themeInitial())
  const railOuvert = ref(false)

  function basculerTheme(): void {
    theme.value = theme.value === 'clair' ? 'sombre' : 'clair'
    appliquerTheme(theme.value)
    retenirTheme(theme.value)
  }

  /** Ouvre ou ferme le rail. Il ne se replie qu'en deçà de la largeur d'un ordinateur. */
  function basculerRail(): void {
    railOuvert.value = !railOuvert.value
  }

  /** Referme le rail. Appelé à chaque navigation : sur téléphone, il couvre l'écran. */
  function fermerRail(): void {
    railOuvert.value = false
  }

  /** Dit à l'opérateur ce qui vient de se passer. */
  function informer(message: string): void {
    toast(message)
  }

  /** Le dit sur le ton d'une panne. */
  function avertir(message: string): void {
    toast.error(message)
  }

  return { theme, railOuvert, avertir, basculerRail, basculerTheme, fermerRail, informer }
})
