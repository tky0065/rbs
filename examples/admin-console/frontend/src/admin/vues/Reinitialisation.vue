<script setup lang="ts">
import { computed, ref } from 'vue'
import { RouterLink, useRoute } from 'vue-router'

import { api, phrase } from '@/api'
import Bande from '@/components/Bande.vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

import { jetonDuLien } from '../lien'
import { TEXTES } from '../textes'

/** Le nom du projet, tel que `rbs new` l'a fixé. */
const PROJET = 'admin-console'

const route = useRoute()

const jeton = computed(() => jetonDuLien(route))

const nouveau = ref('')
const confirmation = ref('')
const enCours = ref(false)
const faute = ref<string | null>(null)
const pose = ref(false)

/**
 * Pose le mot de passe que le jeton du lien autorise.
 *
 * La confirmation est une garde de saisie et non une règle du service : celui-ci ne la
 * connaît pas, et une faute de frappe y enfermerait dehors quelqu'un qui croyait avoir
 * tapé autre chose. La longueur, elle, reste au service — la recopier ici la ferait
 * diverger de la sienne.
 */
async function reinitialiser(): Promise<void> {
  if (nouveau.value !== confirmation.value) {
    faute.value = TEXTES.confirmation_differente
    return
  }

  enCours.value = true
  faute.value = null

  try {
    await api.authResetPassword({ token: jeton.value, new_password: nouveau.value })
    nouveau.value = ''
    confirmation.value = ''
    pose.value = true
  } catch (cause) {
    faute.value = phrase(cause, TEXTES.reinitialisation_refusee)
  } finally {
    enCours.value = false
  }
}
</script>

<template>
  <main class="mx-auto flex min-h-screen max-w-xl flex-col justify-center border-x border-border">
    <Bande>
      <p class="mb-6 uppercase tracking-[0.35em] text-muted-foreground">{{ PROJET }}</p>
      <h1 class="mb-4 text-3xl leading-tight">{{ TEXTES.reinitialisation }}</h1>
      <p class="text-muted-foreground">{{ TEXTES.reinitialisation_detail }}</p>
    </Bande>

    <Bande>
      <!-- Sans jeton, le formulaire ne mène nulle part : l'écran dit ce qui manque plutôt
           que de faire saisir deux fois un mot de passe qu'il ne pourra pas poster. -->
      <p v-if="jeton === ''" class="text-destructive" role="alert">{{ TEXTES.jeton_absent }}</p>

      <p v-else-if="pose" class="text-muted-foreground" role="status">
        {{ TEXTES.reinitialisation_faite }}
      </p>

      <form v-else class="flex flex-col gap-5" novalidate @submit.prevent="reinitialiser">
        <div class="flex flex-col gap-2">
          <Label for="nouveau">{{ TEXTES.nouveau_mot_de_passe }}</Label>
          <Input
            id="nouveau"
            v-model="nouveau"
            type="password"
            autocomplete="new-password"
            required
          />
        </div>

        <div class="flex flex-col gap-2">
          <Label for="confirmation">{{ TEXTES.confirmation }}</Label>
          <Input
            id="confirmation"
            v-model="confirmation"
            type="password"
            autocomplete="new-password"
            required
          />
        </div>

        <p v-if="faute" class="text-destructive" role="alert">{{ faute }}</p>

        <div>
          <Button type="submit" :disabled="enCours">
            {{ enCours ? TEXTES.reinitialisation_en_cours : TEXTES.reinitialiser }}
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
