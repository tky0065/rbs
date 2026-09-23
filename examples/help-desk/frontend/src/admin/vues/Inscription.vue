<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'

import { ApiError, api, phrase } from '@/api'
import Bande from '@/components/Bande.vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

import { TEXTES } from '../textes'

/**
 * Le code que le service met dans le `title` de son refus quand l'inscription est fermée.
 *
 * `auth` rend ses erreurs en RFC 9457, et c'est ce code stable — et non le statut, qu'un
 * 403 d'autre nature porterait aussi — qui distingue une porte close d'une panne.
 */
const FERMEE = 'registration_closed'

/** Le nom du projet, tel que `rbs new` l'a fixé. */
const PROJET = 'help-desk'

const adresse = ref('')
const motDePasse = ref('')
const enCours = ref(false)
const faute = ref<string | null>(null)
const envoye = ref(false)

// `null` tant que le service n'a pas répondu : l'écran ne montre ni le formulaire ni le
// refus avant de savoir lequel des deux il doit. Le réglage est lu par le serveur à son
// démarrage, et une application servie en fichiers statiques n'a que cette route pour le
// connaître.
const ouverte = ref<boolean | null>(null)
const chargement = ref(true)

async function lire(): Promise<void> {
  chargement.value = true

  try {
    ouverte.value = (await api.authRegistrationStatus()).enabled
    faute.value = null
  } catch (cause) {
    ouverte.value = null
    faute.value = phrase(cause, TEXTES.injoignable)
  } finally {
    chargement.value = false
  }
}

async function inscrire(): Promise<void> {
  enCours.value = true
  faute.value = null

  try {
    await api.authRegister({ email: adresse.value, password: motDePasse.value })
    adresse.value = ''
    motDePasse.value = ''
    envoye.value = true
  } catch (cause) {
    // La porte a pu se fermer entre l'ouverture de cet écran et l'envoi : le refus le dit,
    // et l'écran se range alors sur ce que le service vient d'affirmer.
    if (cause instanceof ApiError && cause.problem?.title === FERMEE) {
      ouverte.value = false
    } else {
      faute.value = phrase(cause, TEXTES.inscription_refusee)
    }
  } finally {
    enCours.value = false
  }
}

onMounted(() => {
  void lire()
})
</script>

<template>
  <main class="mx-auto flex min-h-screen max-w-xl flex-col justify-center border-x border-border">
    <Bande>
      <p class="mb-6 uppercase tracking-[0.35em] text-muted-foreground">{{ PROJET }}</p>
      <h1 class="mb-4 text-3xl leading-tight">{{ TEXTES.inscription }}</h1>
      <p class="text-muted-foreground">{{ TEXTES.inscription_sous_titre }}</p>
    </Bande>

    <Bande>
      <p v-if="chargement" class="text-muted-foreground" role="status">
        {{ TEXTES.chargement }}
      </p>

      <!-- Le formulaire disparaît plutôt que de se désactiver : un champ grisé sous une
           phrase de refus laisse croire qu'il suffit d'insister. -->
      <p v-else-if="ouverte === false" class="text-muted-foreground" role="status">
        {{ TEXTES.inscription_fermee }}
      </p>

      <p v-else-if="ouverte === null" class="text-destructive" role="alert">{{ faute }}</p>

      <p v-else-if="envoye" class="text-muted-foreground" role="status">
        {{ TEXTES.inscription_envoyee }}
      </p>

      <form v-else class="flex flex-col gap-5" novalidate @submit.prevent="inscrire">
        <div class="flex flex-col gap-2">
          <Label for="adresse">{{ TEXTES.adresse }}</Label>
          <Input
            id="adresse"
            v-model="adresse"
            type="email"
            name="email"
            autocomplete="username"
            required
          />
        </div>

        <div class="flex flex-col gap-2">
          <Label for="mot-de-passe">{{ TEXTES.mot_de_passe }}</Label>
          <Input
            id="mot-de-passe"
            v-model="motDePasse"
            type="password"
            name="password"
            autocomplete="new-password"
            required
          />
        </div>

        <p v-if="faute" class="text-destructive" role="alert">{{ faute }}</p>

        <div>
          <Button type="submit" :disabled="enCours">
            {{ enCours ? TEXTES.inscription_en_cours : TEXTES.inscrire }}
          </Button>
        </div>
      </form>
    </Bande>

    <Bande>
      <p class="text-muted-foreground">
        {{ TEXTES.deja_inscrit }}
        <RouterLink
          :to="{ name: 'admin-connexion' }"
          class="underline underline-offset-4 hover:text-foreground"
        >
          {{ TEXTES.retour_connexion }}
        </RouterLink>
      </p>
    </Bande>
  </main>
</template>
