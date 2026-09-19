import { existsSync } from 'node:fs'
import { fileURLToPath, URL } from 'node:url'

import tailwindcss from '@tailwindcss/vite'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

// Ce que le serveur de développement ne sert pas lui-même et relaie au binaire : les
// routes que le squelette monte — sonde de santé, interface et document OpenAPI.
//
// Sans ce relais, l'appel partirait sur le port de Vite et rendrait l'application en
// retour. Une route que le projet ajoute — un CRUD engendré, par exemple — se déclare
// ici, faute de quoi elle ne répondra qu'une fois le build en place.
// region: relais
const RELAYE = ['/health', '/docs', '/api-docs']

// Le shell d'administration appelle les routes d'`auth` ; sans ce relais, sa connexion
// partirait sur le port de Vite, qui rendrait l'application en guise de paire de jetons.
//
// La présence du répertoire plutôt qu'une ligne écrite à la pose : le fragment du shell
// arrive peut-être après ce fichier-ci, et il ne peut pas le redéposer. Le routeur
// découvre son montage de la même façon, et le retrait du fragment efface les deux.
if (existsSync(fileURLToPath(new URL('./src/admin', import.meta.url)))) {
  RELAYE.push('/auth')
}
// endregion: relais

// Le port de `[server]` dans `config/default.toml`. Les deux se déplacent ensemble, et
// `RBS_API_URL` ne sert qu'à viser ailleurs le temps d'un lancement : c'est un réglage du
// serveur de développement, que le build ignore comme l'ignore le binaire.
const BINAIRE = process.env.RBS_API_URL ?? 'http://127.0.0.1:8080'

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  server: {
    proxy: Object.fromEntries(
      RELAYE.map((prefixe) => [prefixe, { target: BINAIRE, changeOrigin: true }]),
    ),
  },
})
