import { createPinia } from 'pinia'
import { createApp } from 'vue'

import App from './App.vue'
import { router } from './router'

import './assets/main.css'

// Pinia est monté ici alors qu'aucun store n'existe encore : c'est la couche d'état du
// socle, et l'y poser une fois évite que chaque écran qui viendra s'y ajouter ait à se
// demander si elle est là.
createApp(App).use(createPinia()).use(router).mount('#app')
