<script setup lang="ts">
import type { SessionResponse } from '@/api/client'

import { onMounted, ref } from 'vue'

import { api, phrase } from '@/api'
import Bande from '@/components/Bande.vue'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog'
import {
  Table,
  TableBody,
  TableCell,
  TableEmpty,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { useAuthentification } from '@/stores/authentification'
import { useInterface } from '@/stores/interface'

import { TEXTES } from '../textes'

const authentification = useAuthentification()
const interfaces = useInterface()

const sessions = ref<SessionResponse[]>([])
const enCours = ref(true)
const faute = ref<string | null>(null)

// Les lignes dont la révocation est partie, pour n'éteindre que leurs boutons :
// désactiver toute la table ferait clignoter des lignes qui n'ont rien demandé. Un
// ensemble et non un seul identifiant, parce que rien n'empêche d'en révoquer deux.
const revoquees = ref(new Set<string>())

const toutOuvert = ref(false)
const toutEnCours = ref(false)

async function lire(silencieuse = false): Promise<void> {
  enCours.value = !silencieuse

  try {
    sessions.value = await api.authListSessions()
    faute.value = null
  } catch (cause) {
    sessions.value = []
    faute.value = phrase(cause, TEXTES.sessions_illisibles)
  } finally {
    enCours.value = false
  }
}

async function revoquer(id: string): Promise<void> {
  revoquees.value.add(id)
  faute.value = null

  try {
    await api.authRevokeSession(id)
    interfaces.informer(TEXTES.session_revoquee)
    // Silencieuse : la table reste à l'écran le temps de la relecture. La remplacer par
    // « chargement » serait le clignotement qu'on vient justement d'éviter ligne à ligne.
    await lire(true)
  } catch (cause) {
    faute.value = phrase(cause, TEXTES.revocation_refusee)
  } finally {
    revoquees.value.delete(id)
  }
}

/**
 * Révoque tout, puis sort.
 *
 * Le service ferme toutes les sessions du compte, celle de l'appelant comprise. Rester
 * ici laisserait un écran qui marche encore — le jeton d'accès vit quelques minutes de
 * plus — et cessera de marcher au premier renouvellement, sans que rien ne l'ait dit.
 */
async function revoquerTout(): Promise<void> {
  toutEnCours.value = true
  faute.value = null

  try {
    await api.authRevokeSessions()
    toutOuvert.value = false
    await authentification.deconnexion()
  } catch (cause) {
    faute.value = phrase(cause, TEXTES.revocation_refusee)
  } finally {
    toutEnCours.value = false
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
      <h1 class="mb-4 text-2xl leading-tight">{{ TEXTES.sessions_titre }}</h1>
      <p class="text-muted-foreground">{{ TEXTES.sessions_detail }}</p>
    </Bande>

    <Bande>
      <p v-if="enCours" class="text-muted-foreground" role="status">{{ TEXTES.chargement }}</p>

      <template v-else>
        <Table class="mb-5">
          <TableHeader>
            <TableRow>
              <TableHead>{{ TEXTES.identifiant }}</TableHead>
              <TableHead>{{ TEXTES.ouverte_le }}</TableHead>
              <TableHead>{{ TEXTES.expire_le }}</TableHead>
              <TableHead />
            </TableRow>
          </TableHeader>
          <TableBody>
            <TableRow v-for="session in sessions" :key="session.id">
              <TableCell class="break-all">{{ session.id }}</TableCell>
              <TableCell>{{ date(session.created_at) }}</TableCell>
              <TableCell>{{ date(session.expires_at) }}</TableCell>
              <TableCell class="text-right">
                <Button
                  variant="outline"
                  size="sm"
                  :disabled="revoquees.has(session.id)"
                  @click="revoquer(session.id)"
                >
                  {{
                    revoquees.has(session.id) ? TEXTES.revocation_en_cours : TEXTES.revoquer
                  }}
                </Button>
              </TableCell>
            </TableRow>
            <TableEmpty v-if="sessions.length === 0 && !faute" :colspan="4" class="whitespace-normal">
              {{ TEXTES.aucune_session }}
            </TableEmpty>
          </TableBody>
        </Table>

        <p v-if="faute" class="mb-5 text-destructive" role="alert">{{ faute }}</p>

        <Dialog v-model:open="toutOuvert">
          <DialogTrigger as-child>
            <Button variant="destructive" :disabled="sessions.length === 0">
              {{ TEXTES.revoquer_tout }}
            </Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>{{ TEXTES.revoquer_tout_titre }}</DialogTitle>
              <DialogDescription>{{ TEXTES.revoquer_tout_detail }}</DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="outline" @click="toutOuvert = false">{{ TEXTES.annuler }}</Button>
              <Button variant="destructive" :disabled="toutEnCours" @click="revoquerTout">
                {{ toutEnCours ? TEXTES.revocation_en_cours : TEXTES.revoquer_tout }}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </template>
    </Bande>
  </div>
</template>
