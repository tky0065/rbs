// Capture l'écran d'administration du projet que l'enregistrement vient d'engendrer.
//
// Lancé par `terminal.tape` pendant que `make dev` tourne encore : le binaire et Vite
// meurent avec le terminal de VHS, si bien que la capture ne peut se prendre qu'avant.
// Toute étape qui n'aboutit pas lève, et le piège ERR du terminal fait échouer la
// régénération.

const { chromium } = require('playwright')

const [sortie, adresse, motDePasse] = process.argv.slice(2)
if (!sortie || !adresse || !motDePasse) {
  console.error('usage : capture-admin.cjs <sortie.png> <adresse> <mot de passe>')
  process.exit(2)
}

const VITE = 'http://localhost:5173'

;(async () => {
  const navigateur = await chromium.launch()
  try {
    const page = await navigateur.newPage({
      viewport: { width: 1280, height: 760 },
      deviceScaleFactor: 2,
      colorScheme: 'light',
    })
    page.setDefaultTimeout(30_000)

    await page.goto(`${VITE}/admin/connexion`)
    await page.fill('#adresse', adresse)
    await page.fill('#mot-de-passe', motDePasse)
    await page.click('button[type="submit"]')
    await page.waitForURL(`${VITE}/admin`)

    // Le jeton ne vit qu'en mémoire : un `goto` rechargerait la page et le perdrait, on
    // suit donc le rail comme le ferait un utilisateur.
    await page.getByRole('link', { name: 'Tickets', exact: true }).first().click()
    await page.waitForURL(`${VITE}/admin/tickets`)
    await page.getByRole('cell', { name: 'Printer on fire' }).first().waitFor()

    await page.screenshot({ path: sortie })
  } finally {
    await navigateur.close()
  }
})().catch((erreur) => {
  console.error(erreur)
  process.exit(1)
})
