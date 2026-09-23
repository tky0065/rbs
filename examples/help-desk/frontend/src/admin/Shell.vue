<script setup lang="ts">
import { LogOutIcon, MenuIcon } from '@lucide/vue'
import { watch } from 'vue'
import { RouterLink, RouterView, useRouter } from 'vue-router'

import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'
import { Sheet, SheetContent, SheetHeader, SheetTitle } from '@/components/ui/sheet'
import { useAuthentification } from '@/stores/authentification'
import { useInterface } from '@/stores/interface'

import { RAIL } from './rail'
import { TEXTES } from './textes'

/** Le nom du projet, tel que `rbs new` l'a fixé. */
const PROJET = 'help-desk'

/**
 * La mise d'une entrée du rail, écrite ici parce que les deux rails la portent.
 *
 * Le contour transparent tient la place que prend le halo du focus : posé au moment où
 * il apparaît, il décalerait l'entrée sous le curseur de celui qui l'atteint au clavier.
 */
const ENTREE =
  'flex items-center gap-2 border border-transparent px-2 py-1.5 outline-none hover:bg-accent focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50'

const authentification = useAuthentification()
const interfaces = useInterface()
const routeur = useRouter()

// Toutes les sorties passent par ici, et non par le seul bouton : une session peut aussi
// tomber d'elle-même — renouvellement refusé, session révoquée depuis un autre poste — et
// l'opérateur resterait alors devant un écran mort jusqu'à sa prochaine navigation, la
// garde de route ne se prononçant qu'à ce moment-là.
let volontaire = false

watch(
  () => authentification.connecte,
  (ouverte) => {
    if (ouverte) {
      return
    }

    if (!volontaire) {
      interfaces.avertir(TEXTES.session_expiree)
    }

    void routeur.push({ name: 'admin-connexion' })
  },
)

async function sortir(): Promise<void> {
  volontaire = true
  await authentification.deconnexion()
  interfaces.informer(TEXTES.deconnecte)
}
</script>

<!--
  La coquille : la marge perforée du socle, un rail, et la bande où les écrans viennent se
  monter. La marge est la classe que le socle tient de son thème — sa géométrie n'est donc
  écrite nulle part ici, et réécrire le thème l'efface avec le reste.

  Le rail est servi deux fois — à demeure au-delà de la largeur d'un ordinateur, dans un
  panneau en deçà — et ses deux rendus parcourent la même liste : un écran ajouté d'un
  seul côté serait invisible sur téléphone, sans que rien n'échoue.

  L'entrée courante se marque sur la route exacte et non sur son préfixe : le tableau de
  bord occupe le chemin vide de l'espace, et un marquage par préfixe l'allumerait sur
  toutes les autres. Un écran engendré qui porterait des routes filles aura, lui, à
  marquer la sienne autrement.
-->
<template>
  <div class="flex min-h-screen">
    <aside class="hidden w-60 shrink-0 border-r border-border md:flex">
      <div class="marge shrink-0 border-r border-border" aria-hidden="true" />

      <div class="flex min-w-0 flex-1 flex-col">
        <div class="border-b border-border px-4 py-4">
          <p class="truncate uppercase tracking-[0.3em] text-muted-foreground">{{ PROJET }}</p>
          <p class="truncate text-sm">{{ TEXTES.espace }}</p>
        </div>

        <nav class="flex flex-1 flex-col gap-1 p-3">
          <RouterLink
            v-for="entree in RAIL"
            :key="entree.route"
            :to="{ name: entree.route }"
            :class="ENTREE"
            exact-active-class="bg-accent"
          >
            <component :is="entree.icone" v-if="entree.icone" aria-hidden="true" class="size-4" />
            {{ entree.libelle }}
          </RouterLink>
        </nav>

        <Separator />

        <div class="p-3">
          <p class="truncate px-2 pb-2 text-sm text-muted-foreground">
            {{ authentification.utilisateur?.email }}
          </p>
          <Button variant="outline" size="sm" class="w-full justify-start" @click="sortir">
            <LogOutIcon />
            {{ TEXTES.sortir }}
          </Button>
        </div>
      </div>
    </aside>

    <Sheet v-model:open="interfaces.railOuvert">
      <SheetContent side="left" class="w-60 p-0">
        <SheetHeader class="border-b border-border">
          <SheetTitle class="uppercase tracking-[0.3em]">{{ TEXTES.espace }}</SheetTitle>
        </SheetHeader>
        <nav class="flex flex-col gap-1 p-3" @click="interfaces.fermerRail()">
          <RouterLink
            v-for="entree in RAIL"
            :key="entree.route"
            :to="{ name: entree.route }"
            :class="ENTREE"
            exact-active-class="bg-accent"
          >
            <component :is="entree.icone" v-if="entree.icone" aria-hidden="true" class="size-4" />
            {{ entree.libelle }}
          </RouterLink>
        </nav>
        <div class="mt-auto border-t border-border p-3">
          <Button variant="outline" size="sm" class="w-full justify-start" @click="sortir">
            <LogOutIcon />
            {{ TEXTES.sortir }}
          </Button>
        </div>
      </SheetContent>
    </Sheet>

    <div class="flex min-w-0 flex-1 flex-col">
      <header class="flex items-center gap-3 border-b border-border px-5 py-3 md:hidden">
        <Button
          variant="ghost"
          size="icon"
          :aria-label="TEXTES.menu"
          @click="interfaces.basculerRail()"
        >
          <MenuIcon />
        </Button>
        <p class="truncate uppercase tracking-[0.3em] text-muted-foreground">{{ PROJET }}</p>
      </header>

      <main class="flex-1 px-5 py-6">
        <RouterView />
      </main>
    </div>
  </div>
</template>
