<script setup lang="ts">
import type { UserResponse } from '@/api/client'

import { onMounted, ref } from 'vue'

import { api, phrase } from '@/api'
import Bande from '@/components/Bande.vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useAuthentification } from '@/stores/authentification'
import { useInterface } from '@/stores/interface'

import { TEXTES } from '../textes'

const authentification = useAuthentification()
const interfaces = useInterface()

// Relu plutôt que pris au store : celui-ci porte le compte tel qu'il était à l'ouverture
// de la session, et une adresse prouvée depuis se lirait encore comme ne l'étant pas.
const compte = ref<UserResponse | null>(null)
const enCours = ref(true)
const faute = ref<string | null>(null)

const nouvelleAdresse = ref('')
const adresseEnCours = ref(false)
const fauteAdresse = ref<string | null>(null)

const courant = ref('')
const nouveau = ref('')
const confirmation = ref('')
const changementEnCours = ref(false)
const fauteChangement = ref<string | null>(null)

async function lire(): Promise<void> {
  enCours.value = true

  try {
    compte.value = await api.authMe()
    faute.value = null
  } catch (cause) {
    compte.value = null
    faute.value = phrase(cause, TEXTES.profil_illisible)
  } finally {
    enCours.value = false
  }
}

/**
 * Change l'adresse du compte.
 *
 * Le service répond la même chose que la nouvelle adresse ait été libre ou déjà prise :
 * l'écran ne prétend donc pas savoir laquelle il a touchée, et renvoie vers la boîte de
 * réception. Le compte est relu dans la foulée — la preuve de l'ancienne adresse est
 * tombée, et le laisser affiché comme prouvé mentirait.
 */
async function changerAdresse(): Promise<void> {
  adresseEnCours.value = true
  fauteAdresse.value = null

  try {
    await api.authUpdateMe({ email: nouvelleAdresse.value })
    nouvelleAdresse.value = ''
    interfaces.informer(TEXTES.adresse_envoyee)
    await lire()
  } catch (cause) {
    fauteAdresse.value = phrase(cause, TEXTES.adresse_refusee)
  } finally {
    adresseEnCours.value = false
  }
}

/**
 * Change le mot de passe.
 *
 * La confirmation est une garde de saisie et non une règle du service : celui-ci ne la
 * connaît pas, et une faute de frappe y enfermerait dehors quelqu'un qui croyait avoir
 * tapé autre chose. La longueur, elle, reste au service — la recopier ici la ferait
 * diverger de la sienne.
 */
async function changer(): Promise<void> {
  if (nouveau.value !== confirmation.value) {
    fauteChangement.value = TEXTES.confirmation_differente
    return
  }

  changementEnCours.value = true
  fauteChangement.value = null

  try {
    await authentification.changerMotDePasse(courant.value, nouveau.value)
    courant.value = ''
    nouveau.value = ''
    confirmation.value = ''
    interfaces.informer(TEXTES.mot_de_passe_change)
  } catch (cause) {
    fauteChangement.value = phrase(cause, TEXTES.changement_refuse)
  } finally {
    changementEnCours.value = false
  }
}

/** L'horodatage dans les conventions du navigateur : ce sont celles de l'opérateur. */
function date(valeur: string): string {
  return new Date(valeur).toLocaleString()
}

onMounted(() => {
  void lire()
})
</script>

<template>
  <div class="feuille mx-auto max-w-4xl border-x border-border px-4 sm:px-6">
    <Bande>
      <h1 class="text-2xl leading-tight">{{ TEXTES.profil }}</h1>
    </Bande>

    <Bande :titre="TEXTES.compte">
      <p v-if="enCours" class="text-muted-foreground" role="status">{{ TEXTES.chargement }}</p>

      <dl v-else-if="compte" class="grid gap-x-6 gap-y-1 sm:grid-cols-[minmax(10rem,auto)_1fr]">
        <dt class="text-muted-foreground">{{ TEXTES.adresse }}</dt>
        <dd class="m-0 break-all">{{ compte.email }}</dd>
        <dt class="text-muted-foreground max-sm:mt-3">{{ TEXTES.role }}</dt>
        <dd class="m-0">{{ compte.role }}</dd>
        <dt class="text-muted-foreground max-sm:mt-3">{{ TEXTES.inscrit_le }}</dt>
        <dd class="m-0">{{ date(compte.created_at) }}</dd>
        <dt class="text-muted-foreground max-sm:mt-3">{{ TEXTES.verification }}</dt>
        <dd class="m-0" :class="compte.email_verified_at ? '' : 'text-destructive'">
          {{ compte.email_verified_at ? date(compte.email_verified_at) : TEXTES.adresse_non_verifiee }}
        </dd>
      </dl>

      <p v-else class="text-destructive" role="alert">{{ faute }}</p>
    </Bande>

    <Bande :titre="TEXTES.adresse_titre">
      <p class="mb-6 text-muted-foreground">{{ TEXTES.adresse_detail }}</p>

      <form class="flex max-w-md flex-col gap-5" novalidate @submit.prevent="changerAdresse">
        <div class="flex flex-col gap-2">
          <Label for="nouvelle-adresse">{{ TEXTES.nouvelle_adresse }}</Label>
          <Input
            id="nouvelle-adresse"
            v-model="nouvelleAdresse"
            type="email"
            autocomplete="username"
            required
          />
        </div>

        <p v-if="fauteAdresse" class="text-destructive" role="alert">{{ fauteAdresse }}</p>

        <div>
          <Button type="submit" :disabled="adresseEnCours">
            {{ adresseEnCours ? TEXTES.envoi_en_cours : TEXTES.changer }}
          </Button>
        </div>
      </form>
    </Bande>

    <Bande :titre="TEXTES.mot_de_passe_titre">
      <p class="mb-6 text-muted-foreground">{{ TEXTES.mot_de_passe_detail }}</p>

      <form class="flex max-w-md flex-col gap-5" novalidate @submit.prevent="changer">
        <div class="flex flex-col gap-2">
          <Label for="courant">{{ TEXTES.mot_de_passe_courant }}</Label>
          <Input
            id="courant"
            v-model="courant"
            type="password"
            autocomplete="current-password"
            required
          />
        </div>

        <div class="flex flex-col gap-2">
          <Label for="nouveau">{{ TEXTES.nouveau_mot_de_passe }}</Label>
          <Input id="nouveau" v-model="nouveau" type="password" autocomplete="new-password" required />
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

        <p v-if="fauteChangement" class="text-destructive" role="alert">{{ fauteChangement }}</p>

        <div>
          <Button type="submit" :disabled="changementEnCours">
            {{ changementEnCours ? TEXTES.changement_en_cours : TEXTES.changer }}
          </Button>
        </div>
      </form>
    </Bande>
  </div>
</template>
