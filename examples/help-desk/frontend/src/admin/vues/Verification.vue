<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { RouterLink, useRoute } from 'vue-router'

import { api, phrase } from '@/api'
import Bande from '@/components/Bande.vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

import { jetonDuLien } from '../lien'
import { TEXTES } from '../textes'

/** Le nom du projet, tel que `rbs new` l'a fixé. */
const PROJET = 'help-desk'

const route = useRoute()

const enCours = ref(false)
const prouvee = ref(false)
const faute = ref<string | null>(null)

const adresse = ref('')
const renvoiEnCours = ref(false)
const renvoiFaute = ref<string | null>(null)
const renvoye = ref(false)

/**
 * Poste le jeton du lien, sans rien demander.
 *
 * L'opérateur vient de cliquer : lui présenter un bouton pour confirmer ce clic ferait
 * deux gestes là où le courriel en promettait un.
 */
async function prouver(jeton: string): Promise<void> {
  enCours.value = true
  faute.value = null

  try {
    await api.authVerifyEmail({ token: jeton })
    prouvee.value = true
  } catch (cause) {
    faute.value = phrase(cause, TEXTES.preuve_refusee)
  } finally {
    enCours.value = false
  }
}

/**
 * Redemande un lien.
 *
 * Le rattrapage d'un jeton périmé, consommé, ou parti dans un courriel jamais arrivé. Le
 * service répond la même chose que l'adresse attende sa preuve ou non : l'écran ne
 * prétend donc pas savoir laquelle il a touchée.
 */
async function renvoyer(): Promise<void> {
  renvoiEnCours.value = true
  renvoiFaute.value = null

  try {
    await api.authResendVerification({ email: adresse.value })
    adresse.value = ''
    renvoye.value = true
  } catch (cause) {
    renvoiFaute.value = phrase(cause, TEXTES.injoignable)
  } finally {
    renvoiEnCours.value = false
  }
}

onMounted(() => {
  const jeton = jetonDuLien(route)

  if (jeton === '') {
    faute.value = TEXTES.jeton_absent
    return
  }

  void prouver(jeton)
})
</script>

<template>
  <main class="mx-auto flex min-h-screen max-w-xl flex-col justify-center border-x border-border">
    <Bande>
      <p class="mb-6 uppercase tracking-[0.35em] text-muted-foreground">{{ PROJET }}</p>
      <h1 class="mb-4 text-3xl leading-tight">{{ TEXTES.preuve }}</h1>
      <p class="text-muted-foreground">{{ TEXTES.preuve_detail }}</p>
    </Bande>

    <Bande>
      <p v-if="enCours" class="text-muted-foreground" role="status">
        {{ TEXTES.preuve_en_cours }}
      </p>
      <p v-else-if="prouvee" class="text-muted-foreground" role="status">
        {{ TEXTES.preuve_faite }}
      </p>
      <p v-else class="text-destructive" role="alert">{{ faute }}</p>
    </Bande>

    <!-- Le renvoi reste offert même quand la preuve est faite : rien ne dit que l'adresse
         de cet onglet est celle que l'opérateur voulait prouver. -->
    <Bande :titre="TEXTES.renvoyer">
      <p v-if="renvoye" class="text-muted-foreground" role="status">{{ TEXTES.renvoye }}</p>

      <form v-else class="flex max-w-md flex-col gap-5" novalidate @submit.prevent="renvoyer">
        <div class="flex flex-col gap-2">
          <Label for="renvoi-adresse">{{ TEXTES.adresse }}</Label>
          <Input
            id="renvoi-adresse"
            v-model="adresse"
            type="email"
            autocomplete="username"
            required
          />
        </div>

        <p v-if="renvoiFaute" class="text-destructive" role="alert">{{ renvoiFaute }}</p>

        <div>
          <Button type="submit" :disabled="renvoiEnCours">
            {{ renvoiEnCours ? TEXTES.envoi_en_cours : TEXTES.renvoyer }}
          </Button>
        </div>
      </form>
    </Bande>

    <Bande>
      <RouterLink
        :to="{ name: 'admin-connexion' }"
        class="underline underline-offset-4 hover:text-foreground"
      >
        {{ TEXTES.retour_connexion }}
      </RouterLink>
    </Bande>
  </main>
</template>
