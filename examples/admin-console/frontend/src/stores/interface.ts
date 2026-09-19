import { defineStore } from 'pinia'
import { ref } from 'vue'
import { toast } from 'vue-sonner'

/** La clé du stockage local, préfixée du nom du projet comme celle de la session. */
const CLE = 'admin-console.theme'

/**
 * Les deux jeux de valeurs que le bloc de thème peut porter.
 *
 * Celui qui est livré n'en porte qu'un : la variante sombre est accrochée à la classe
 * `sombre` et n'a rien à afficher tant qu'on ne lui a pas écrit ses valeurs. Le réglage
 * existe ici parce que c'est l'état de l'interface, et qu'un écran n'a pas à savoir
 * comment on le conserve ; l'affordance qui le bascule viendra avec la seconde palette.
 */
export type Theme = 'clair' | 'sombre'

/**
 * L'état de l'interface : son thème, son rail, et ce qu'elle dit à l'opérateur.
 *
 * Le second et dernier store de l'application. Ce qui vit ici n'appartient à aucun écran
 * en particulier et survit à leur navigation — le reste est de l'état de composant.
 */
export const useInterface = defineStore('interface', () => {
  const theme = ref<Theme>(lu())
  const railOuvert = ref(false)

  appliquer(theme.value)

  function basculerTheme(): void {
    theme.value = theme.value === 'clair' ? 'sombre' : 'clair'
    appliquer(theme.value)

    try {
      window.localStorage.setItem(CLE, theme.value)
    } catch {
      // Sans stockage, le choix ne vaut que pour cet onglet.
    }
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

/** Le thème retenu au dernier passage, clair à défaut. */
function lu(): Theme {
  try {
    return window.localStorage.getItem(CLE) === 'sombre' ? 'sombre' : 'clair'
  } catch {
    return 'clair'
  }
}

/** Accroche la classe que le bloc de thème attend, sur la racine du document. */
function appliquer(theme: Theme): void {
  document.documentElement.classList.toggle('sombre', theme === 'sombre')
}
