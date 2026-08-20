// BabyEng 运行验证脚本：用 playwright-core 连系统 Chrome，逐页截图 + 冒烟断言
// 用法: node verify.js [baseUrl]
const { chromium } = require('playwright-core')

const BASE = process.argv[2] || 'http://127.0.0.1:8080'
const OUT = process.argv[3] || '/tmp/babyeng-shots'
const USERNAME = process.env.AUTH_USERNAME
const PASSWORD = process.env.AUTH_PASSWORD

const pages = [
  { path: '/onboarding', name: '01-onboarding' },
  { path: '/home', name: '02-home' },
  { path: '/ask', name: '03-ask' },
  { path: '/compare?target_type=word&target_id=word_cup&en=cup&zh=杯子&phonetic=%2Fk%CA%8Cp%2F&emoji=%E2%98%95', name: '04-compare' },
  { path: '/audio', name: '05-audio' },
  { path: '/settings', name: '06-settings' },
  { path: '/states', name: '07-states' },
  { path: '/learn', name: '08-learn' },
  { path: '/word-learn?category=item', name: '09-wordlearn' },
  { path: '/review', name: '10-review' },
  { path: '/sentences', name: '11-sentences' },
  { path: '/profile', name: '12-profile' },
]

async function main() {
  if (!USERNAME || !PASSWORD) throw new Error('请通过 AUTH_USERNAME 和 AUTH_PASSWORD 提供验收账号')
  const fs = require('fs')
  fs.mkdirSync(OUT, { recursive: true })
  const browser = await chromium.launch({
    executablePath: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    headless: true,
  })
  const ctx = await browser.newContext({
    viewport: { width: 414, height: 896 },
    deviceScaleFactor: 2,
    isMobile: true,
    hasTouch: true,
  })
  const results = []

  // 先走真实登录页，后续页面复用 localStorage 中的会话。
  {
    const page = await ctx.newPage()
    await page.goto(BASE + '/login', { waitUntil: 'networkidle', timeout: 15000 })
    await page.locator('input[autocomplete="username"]').fill(USERNAME)
    await page.locator('input[autocomplete="current-password"]').fill(PASSWORD)
    await page.getByRole('button', { name: '登录' }).click()
    await page.waitForURL(/\/(home|onboarding)$/, { timeout: 15000 })
    results.push({ page: 'login', ok: true })
    await page.close()
  }

  // 前置：通过 API 初始化一个家庭（否则页面会跳引导页）
  {
    const page = await ctx.newPage()
    try {
      await page.goto(BASE + '/', { waitUntil: 'domcontentloaded', timeout: 15000 })
      const init = await page.evaluate(async () => {
        const r = await fetch('/api/family/init', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${localStorage.getItem('babyeng_auth_token')}` },
          body: JSON.stringify({ child_name: '糖糖', child_birthdate: '2024-10-01' }),
        })
        return r.json()
      })
      results.push({ page: 'init-family', ok: !!init.family_id, band: init.age_band })
    } catch (e) {
      results.push({ page: 'init-family', ok: false, errors: [e.message] })
    }
    await page.close()
  }

  for (const p of pages) {
    const page = await ctx.newPage()
    const errors = []
    page.on('pageerror', (e) => errors.push('pageerror: ' + e.message))
    page.on('console', (m) => {
      if (m.type() === 'error') errors.push('console: ' + m.text())
    })
    try {
      await page.goto(BASE + p.path, { waitUntil: 'networkidle', timeout: 20000 })
      await page.waitForTimeout(600)
      const overflow = await page.evaluate(() => {
        const doc = document.documentElement
        return doc.scrollWidth > doc.clientWidth + 2
      })
      await page.screenshot({ path: `${OUT}/${p.name}.png`, fullPage: false })
      results.push({ page: p.name, ok: !overflow && errors.length === 0, overflow, errors: errors.slice(0, 3) })
    } catch (e) {
      results.push({ page: p.name, ok: false, errors: ['load: ' + e.message] })
    }
    await page.close()
  }

  // 交互冒烟：Ask 页文本提问「杯子」→ 应命中 cup
  {
    const page = await ctx.newPage()
    try {
      await page.goto(BASE + '/ask', { waitUntil: 'networkidle', timeout: 20000 })
      await page.waitForTimeout(600)
      // 点「改用打字」展开输入
      await page.getByRole('button', { name: /改用打字/ }).click()
      await page.waitForTimeout(200)
      await page.locator('.textinput input').first().fill('杯子')
      await page.locator('.textinput .btn').first().click()
      await page.waitForTimeout(1500)
      const en = await page.locator('.word-block .t-word-en').first().textContent().catch(() => null)
      await page.screenshot({ path: `${OUT}/smoke-ask-text-cup.png` })
      results.push({ page: 'smoke-ask-cup', ok: en === 'cup', got: en })
    } catch (e) {
      results.push({ page: 'smoke-ask-cup', ok: false, errors: [e.message] })
    }
    await page.close()
  }

  // 交互冒烟：未命中 → nomatch 态 + 推荐词
  {
    const page = await ctx.newPage()
    try {
      await page.goto(BASE + '/ask', { waitUntil: 'networkidle', timeout: 20000 })
      await page.waitForTimeout(600)
      await page.getByRole('button', { name: /改用打字/ }).click()
      await page.waitForTimeout(200)
      await page.locator('.textinput input').first().fill('妈妈的口红')
      await page.locator('.textinput .btn').first().click()
      await page.waitForTimeout(1500)
      const hasBanner = await page.locator('.banner.warn').count()
      const suggestCount = await page.locator('.suggest button').count()
      await page.screenshot({ path: `${OUT}/smoke-ask-nomatch.png` })
      results.push({ page: 'smoke-ask-nomatch', ok: hasBanner > 0 && suggestCount > 0, banner: hasBanner, suggest: suggestCount })
    } catch (e) {
      results.push({ page: 'smoke-ask-nomatch', ok: false, errors: [e.message] })
    }
    await page.close()
  }

  // 交互冒烟：二选一（问「被子」应 ambiguous 或 hit）
  {
    const page = await ctx.newPage()
    try {
      await page.goto(BASE + '/ask', { waitUntil: 'networkidle', timeout: 20000 })
      await page.waitForTimeout(600)
      await page.getByRole('button', { name: /改用打字/ }).click()
      await page.waitForTimeout(200)
      await page.locator('.textinput input').first().fill('被子')
      await page.locator('.textinput .btn').first().click()
      await page.waitForTimeout(1500)
      const choiceCount = await page.locator('.choice button').count()
      const wordEn = await page.locator('.word-block .t-word-en').first().textContent().catch(() => null)
      await page.screenshot({ path: `${OUT}/smoke-ask-ambiguous.png` })
      results.push({ page: 'smoke-ask-ambiguous', ok: choiceCount >= 1 || wordEn === 'cup', choices: choiceCount, word: wordEn })
    } catch (e) {
      results.push({ page: 'smoke-ask-ambiguous', ok: false, errors: [e.message] })
    }
    await page.close()
  }

  // 数据冒烟：直接调 API
  {
    const page = await ctx.newPage()
    try {
      await page.goto(BASE + '/', { waitUntil: 'domcontentloaded', timeout: 15000 })
      const words = await page.evaluate(async () => (await fetch('/api/words')).json())
      const scenes = await page.evaluate(async () => (await fetch('/api/scenes')).json())
      const ready = await page.evaluate(async () => (await fetch('/api/readyz')).json())
      const sentences = await page.evaluate(async () => (await fetch('/api/sentences')).json())
      results.push({
        page: 'api-words',
        ok: (words.words || []).length === 48,
        words: words.words.length,
      })
      results.push({
        page: 'api-sentences',
        ok: (sentences.sentences || []).length === 10,
        sentences: sentences.sentences.length,
      })
      results.push({
        page: 'api-scenes',
        ok: (scenes.scenes || []).length >= 4,
        scenes: scenes.scenes.length,
      })
      results.push({
        page: 'api-readyz',
        ok: !!ready.ok,
        svc: JSON.stringify(ready.services),
      })
    } catch (e) {
      results.push({ page: 'api-smoke', ok: false, errors: [e.message] })
    }
    await page.close()
  }

  await browser.close()
  console.log(JSON.stringify(results, null, 2))
  const failed = results.filter((r) => !r.ok)
  if (failed.length) {
    console.log(`\nFAILED: ${failed.length}/${results.length}`)
    process.exit(1)
  }
  console.log(`\nALL PASS (${results.length})`)
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})
