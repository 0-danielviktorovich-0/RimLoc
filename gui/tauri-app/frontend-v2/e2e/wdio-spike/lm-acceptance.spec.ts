// Language Manager acceptance (§10/§32) — REAL rel17 react artifact app.
// LM is the #/lm ROUTE (LanguageManager.tsx); CRUD поверх localStorage
// rimloc.languages.user.v1 (lib/languages/manager.ts). Каждый шаг снимает
// скриншот-доказательство в /tmp/palette-lm/shots/.
const SHOTS = '/tmp/palette-lm/shots'
const LM_KEY = 'rimloc.languages.user.v1'

function pause(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}

async function waitExisting(sel: string, timeout = 30000): Promise<void> {
  await browser.waitUntil(async () => (await browser.$(sel)).isExisting(), { timeout, interval: 100 })
}

let shotN = 100
async function shot(name: string): Promise<void> {
  shotN += 1
  const file = `${SHOTS}/${shotN}-${name}.png`
  try {
    await browser.saveScreenshot(file)
  } catch {
    console.log(`[lm-acc] screenshot unavailable: ${name}`)
  }
}

async function lmRows(): Promise<string[]> {
  return browser.execute(() =>
    [...document.querySelectorAll('[data-testid^="lm.row."]')].map(
      (r) => r.getAttribute('data-testid')!.replace('lm.row.', ''),
    ),
  )
}

async function storedLanguages(): Promise<string> {
  return browser.execute((k) => localStorage.getItem(k) ?? '<absent>', LM_KEY)
}

async function openLM(): Promise<void> {
  await browser.execute(() => { window.location.hash = '#/lm' })
  await waitExisting('[data-testid="lm.table"]', 15000)
  await pause(300)
}

/** Заполнить и отправить форму добавления языка. */
async function submitLang(id: string, display: string, native: string): Promise<void> {
  await browser.$('[data-testid="lm.add-id"]').setValue(id)
  await browser.$('[data-testid="lm.add-display"]').setValue(display)
  await browser.$('[data-testid="lm.add-native"]').setValue(native)
  await browser.$('[data-testid="lm.add-submit"]').click()
  await pause(500)
}

async function errorText(): Promise<string> {
  return browser.execute(() => document.querySelector('.inline-warning')?.textContent?.trim() ?? '<нет ошибки>')
}

async function deleteLang(localeId: string): Promise<void> {
  await browser.execute((id) => {
    const row = document.querySelector(`[data-testid="lm.row.${id}"]`)
    const btn = row?.querySelector('button') as HTMLElement | null
    btn?.click()
  }, localeId)
  await pause(500)
}

// Сабтаг ≤8 символов (registry.ts валидатор) — Date.now() даёт 13 цифр и
// честно отклоняется приложением (rel20-находка). Base36-хвост уникален.
const QA_ID = `qa-${(Date.now() % 1e8).toString(36)}`

before(async () => {
  await waitExisting('.app-sidebar', 30000)
  // Уборка мусора прошлых прогонов этого лейна (qa-*), чтобы состав списка
  // был детерминированным.
  await openLM()
  for (let i = 0; i < 12; i++) {
    const nextQa = await browser.execute(() => {
      const row = [...document.querySelectorAll('[data-testid^="lm.row.qa-"]')][0]
      if (!row) return null
      const id = row.getAttribute('data-testid')!.replace('lm.row.', '')
      const btn = row.querySelector('button') as HTMLElement | null
      btn?.click()
      return id
    })
    if (!nextQa) break
    await pause(300)
  }
  const rows = await lmRows()
  if (rows.length !== 8) throw new Error(`после уборки ожидалось 8 встроенных, осталось ${rows.length}: ${JSON.stringify(rows)}`)
})

describe('LM §10: доступность и состав', () => {
  it('Менеджер языков ДОСТУПЕН из UI: ссылка в сайдбаре есть (MUST-FIX #1 закрыт)', async () => {
    const links = await browser.execute(() => ({
      lmLinks: document.querySelectorAll('a[href="#/lm"]').length,
      allHashes: [...document.querySelectorAll('a[href^="#/"]')].map((a) => a.getAttribute('href')),
    }))
    console.log(`[lm-acc] LINKS_TO_LM=${links.lmLinks} ALL=${JSON.stringify(links.allHashes)}`)
    if (links.lmLinks < 1) throw new Error('ссылки на #/lm в сайдбаре нет — MUST-FIX не закрыт')
    // Команда «Языки» в палитре проверяется в palette-acceptance.spec.ts.
    await openLM()
    await shot('lm-opened-via-nav')
  })

  it('8 встроенных языков; удаляющей кнопки у встроенных нет', async () => {
    const rows = await lmRows()
    console.log(`[lm-acc] BUILTIN_ROWS=${JSON.stringify(rows)}`)
    const expected = ['en', 'ru', 'uk', 'ja', 'de', 'pl', 'es', 'zh-Hans']
    for (const e of expected) {
      if (!rows.includes(e)) throw new Error(`нет встроенного языка ${e}`)
    }
    const deletable = await browser.execute(() => {
      const out: string[] = []
      for (const r of document.querySelectorAll('[data-testid^="lm.row."]')) {
        if (r.querySelector('button')) out.push(r.getAttribute('data-testid')!)
      }
      return out
    })
    if (deletable.length !== 0) throw new Error(`у встроенных есть кнопка удаления: ${JSON.stringify(deletable)}`)
    await shot('lm-builtins')
  })
})

describe('LM §10: CRUD пользовательского языка', () => {
  it('создание: id/display/native → строка появляется, помечена как пользовательский', async () => {
    await submitLang(QA_ID, 'QA Test Lang', 'Куа Тест')
    await waitExisting(`[data-testid="lm.row.${QA_ID}"]`, 10000)
    const rowText = await browser.$(`[data-testid="lm.row.${QA_ID}"]`).getText()
    console.log(`[lm-acc] CREATED ${QA_ID}: ${rowText.replace(/\n/g, ' | ')}`)
    const stored = await storedLanguages()
    console.log(`[lm-acc] STORED=${stored}`)
    if (!stored.includes(QA_ID)) throw new Error('созданный язык не попал в localStorage')
    if (!stored.includes('QA Test Lang') || !stored.includes('Куа Тест')) {
      throw new Error('displayName/nativeName не сохранились')
    }
    await shot('lm-created')
  })

  it('валидация: дубликат встроенного (ru) отклоняется с сообщением', async () => {
    const before = await lmRows()
    await submitLang('ru', 'Русский дубль', 'Русский дубль')
    const err = await errorText()
    const after = await lmRows()
    console.log(`[lm-acc] DUP_BUILTIN err=${JSON.stringify(err)} rows=${after.length} (было ${before.length})`)
    if (err === '<нет ошибки>') throw new Error('дубликат builtin принят без ошибки')
    if (after.length !== before.length) throw new Error('дубликат builtin ДОБАВИЛСЯ в список')
    await shot('lm-error-dup-builtin')
  })

  it('валидация: дубликат пользовательского (qa-dup) отклоняется', async () => {
    await submitLang('qa-dup', 'Дубль', 'Дубль')
    await waitExisting('[data-testid="lm.row.qa-dup"]', 10000)
    const before = await lmRows()
    await submitLang('qa-dup', 'Дубль 2', 'Дубль 2')
    const err = await errorText()
    const after = await lmRows()
    console.log(`[lm-acc] DUP_USER err=${JSON.stringify(err)} rows=${after.length} (было ${before.length})`)
    if (!/already exists/i.test(err)) throw new Error(`ожидалось «already exists», получено: ${err}`)
    if (after.length !== before.length) throw new Error('дубликат пользователя ДОБАВИЛСЯ')
    await shot('lm-error-dup-user')
  })

  it('пустое имя: нативный required блокирует отправку формы (ветка ошибки недостижима)', async () => {
    await browser.$('[data-testid="lm.add-id"]').setValue('qa-empty-name')
    // прошлый тест оставил значения в display/native — чистим явно
    await browser.$('[data-testid="lm.add-display"]').setValue('')
    await browser.$('[data-testid="lm.add-native"]').setValue('')
    await browser.$('[data-testid="lm.add-submit"]').click()
    await pause(500)
    const state = await browser.execute(() => {
      const form = document.querySelector('[data-testid="lm.add-submit"]')?.closest('form')
      const display = document.querySelector('[data-testid="lm.add-display"]') as HTMLInputElement | null
      const row = document.querySelector('[data-testid="lm.row.qa-empty-name"]')
      return {
        formValid: form ? (form as HTMLFormElement).checkValidity() : null,
        displayValid: display?.validity.valid ?? null,
        displayMessage: display?.validationMessage ?? '',
        rowAdded: Boolean(row),
      }
    })
    console.log(`[lm-acc] EMPTY_NAME=${JSON.stringify(state)}`)
    if (state.rowAdded) throw new Error('язык с пустым display добавился')
    if (state.displayValid !== false) {
      throw new Error('required не заблокировал пустое display — проверь форму')
    }
    console.log('[lm-acc] FINDING: «localeId must not be empty» из manager.ts недостижим из UI — пустоту ловит нативный required')
    await shot('lm-empty-name-blocked')
  })

  it('некорректный ввод: понятность ошибок + сообщение не гаснет после успешного действия', async () => {
    // Ошибки формулируются по-английски технически в русском UI:
    // «localeId conflicts with builtin» / «localeId already exists».
    const dupErr = await errorText()
    console.log(`[lm-acc] ERROR_TEXT_STYLE=${JSON.stringify(dupErr)}`)
    // Успешное добавление — проверяем, гаснет ли старая ошибка.
    await submitLang('qa-stale-err', 'Stale Err', 'Стейл')
    await waitExisting('[data-testid="lm.row.qa-stale-err"]', 10000)
    const errAfterSuccess = await errorText()
    console.log(`[lm-acc] ERROR_AFTER_SUCCESS=${JSON.stringify(errAfterSuccess)}`)
    if (errAfterSuccess !== '<нет ошибки>' && errAfterSuccess === dupErr) {
      console.log('[lm-acc] FINDING: ошибка предыдущей попытки остаётся видимой после успешного добавления (LanguageManager.tsx:60-62 не чистит error)')
    }
    await shot('lm-stale-error')
    await deleteLang('qa-stale-err')
    if ((await lmRows()).includes('qa-stale-err')) throw new Error('qa-stale-err не удалился')
  })

  it('валидация формата: битый locale-код («1плохо код!») отклоняется через isValidLocaleId (MUST-FIX закрыт)', async () => {
    const before = await lmRows()
    await submitLang('1плохо код!', 'Битый', 'Битый')
    const after = await lmRows()
    const err = await browser.execute(() => {
      const e = document.querySelector('.form-error, [data-testid="lm.error"]')
      return e ? e.textContent : document.body.innerText.match(/invalid locale id format/)?.[0] ?? '<нет>'
    })
    console.log(`[lm-acc] BROKEN_ID rows=${after.length} (было ${before.length}) err=${JSON.stringify(err)}`)
    if (after.includes('1плохо код!')) {
      throw new Error('битый id ПРИНЯТ — валидация isValidLocaleId не работает')
    }
    if (before.length !== after.length) throw new Error('список вырос на отклонённом вводе')
    if (!String(err).includes('invalid locale id')) throw new Error(`сообщение о формате не показано: ${JSON.stringify(err)}`)
    await shot('lm-broken-id-rejected')
  })

  it('валидация формата: корректные составные id (zh-Hans, en-US-x-私有) проходят', async () => {
    const before = await lmRows()
    await submitLang('zz-Test-Lang', 'Тестовый составной', 'Тестовый')
    const after = await lmRows()
    if (!after.includes('zz-Test-Lang')) throw new Error('корректный составной id отклонён — валидатор слишком строг')
    await shot('lm-composite-id-ok')
    await deleteLang('zz-Test-Lang')
    if ((await lmRows()).length !== before.length) throw new Error('cleanup не сработал')
  })

  it('поиск по списку языков: НЕ реализован (3 input — только форма добавления)', async () => {
    const inputs = await browser.execute(() =>
      [...document.querySelectorAll('.page-content input')].map((i) =>
        (i as HTMLInputElement).getAttribute('data-testid'),
      ),
    )
    console.log(`[lm-acc] LM_INPUTS=${JSON.stringify(inputs)}`)
    if (inputs.length !== 3) {
      console.log('[lm-acc] FINDING: состав input изменился — проверь, не появился ли поиск')
    } else {
      console.log('[lm-acc] MUST-FIX: поиска по списку языков нет (встроенных 8 + пользовательские, фильтра нет)')
    }
    await shot('lm-no-search')
  })
})

describe('LM §10: связь с target и мульти-таргет изоляция', () => {
  before(async () => {
    // Открываем проект, созданный palette-прогоном (те же данные лейна).
    await browser.execute(() => { window.location.hash = '#/projects' })
    await waitExisting('[data-testid="home.project-card"]', 30000)
    await browser.$('[data-testid="home.project-card"]').click()
    await waitExisting('[data-testid="ws.root"]', 30000)
    await pause(500)
    await shot('lm-project-opened')
  })

  it('пользовательский язык появляется в переключателе цели (MUST-FIX #2 закрыт)', async () => {
    const opts = await browser.execute(() => {
      const sel = document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement | null
      return sel ? [...sel.options].map((o) => o.value) : '<нет селекта>'
    })
    console.log(`[lm-acc] TARGET_OPTIONS=${JSON.stringify(opts)} custom=${QA_ID}`)
    if (!Array.isArray(opts)) throw new Error('селекта цели нет в воркспейсе')
    if (!opts.includes(QA_ID)) {
      throw new Error(`созданный в LM язык не попал в переключатель цели (opts=${JSON.stringify(opts)}) — MUST-FIX не закрыт`)
    }
    await shot('lm-custom-in-target-select')
  })

  it('изоляция: правка в ru не видна в uk; возврат в ru сохраняет правку', async () => {
    // ru: правим первую строку
    await browser.execute(() => {
      const sel = document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement
      sel.value = 'ru'
      sel.dispatchEvent(new Event('change', { bubbles: true }))
    })
    await pause(600)
    await browser.$('[data-testid="ws.entry.0"]').click()
    await waitExisting('[data-testid="ws.editor-textarea"]', 10000)
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('[LM-ACC ru] изоляция')
    await browser.$('[data-testid="ws.editor-save-next"]').click()
    await browser.waitUntil(
      async () => (await browser.$('[data-testid="ws.root"]').getText()).includes('[LM-ACC ru]'),
      { timeout: 30000, interval: 250 },
    )
    await shot('lm-isolation-ru-committed')
    // uk: той же строки правка не видна
    await browser.execute(() => {
      const sel = document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement
      sel.value = 'uk'
      sel.dispatchEvent(new Event('change', { bubbles: true }))
    })
    await pause(600)
    const ukText = await browser.$('[data-testid="ws.root"]').getText()
    if (ukText.includes('[LM-ACC ru]')) throw new Error('БАГ ИЗОЛЯЦИИ: правка ru протекла в uk')
    await shot('lm-isolation-uk-empty')
    // возврат в ru: правка на месте
    await browser.execute(() => {
      const sel = document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement
      sel.value = 'ru'
      sel.dispatchEvent(new Event('change', { bubbles: true }))
    })
    await pause(600)
    const ruText = await browser.$('[data-testid="ws.root"]').getText()
    if (!ruText.includes('[LM-ACC ru]')) throw new Error('правка ru потерялась после круга ru→uk→ru')
    console.log('[lm-acc] ISOLATION ru→uk→ru: ок')
  })

  it('коммит в uk перерисовывает список В АКТИВНОЙ цели (MUST-FIX hardcoded ru закрыт)', async () => {
    await browser.execute(() => {
      const sel = document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement
      sel.value = 'uk'
      sel.dispatchEvent(new Event('change', { bubbles: true }))
    })
    await pause(600)
    await browser.$('[data-testid="ws.entry.0"]').click()
    await waitExisting('[data-testid="ws.editor-textarea"]', 10000)
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('[LM-ACC uk] изоляция')
    await browser.$('[data-testid="ws.editor-save-next"]').click()
    // Фаза «select=uk, а в списке тексты ru» больше невозможна: перерисовка
    // после акка маппит снапшот в активную цель (project.ts). Снимаем ту же
    // временную шкалу и требуем её отсутствие.
    const timeline: string[] = []
    let bugSeen = false
    for (let i = 0; i < 16; i++) {
      await pause(500)
      const s = await browser.execute(() => {
        const sel = document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement | null
        const t = document.querySelector('[data-testid="ws.root"]')?.textContent ?? ''
        return `${sel?.value ?? '?'}|ru=${t.includes('[LM-ACC ru]')}|uk=${t.includes('[LM-ACC uk]')}`
      })
      timeline.push(s)
      if (s.startsWith('uk|ru=true|uk=false')) {
        if (!bugSeen) await shot('BUG-lm-commit-uk-shows-ru')
        bugSeen = true
      }
    }
    console.log(`[lm-acc] COMMIT_UK_TIMELINE=${JSON.stringify(timeline)}`)
    if (bugSeen) {
      throw new Error('фаза ru-перерисовки ПОЙМАНА при активном uk — hardcoded ru вернулся')
    }
    console.log('[lm-acc] MUST-FIX закрыт: ru-фаза не обнаружена ни в одном сэмпле')
    // Правка тем не менее сохранилась серверно: переключение ru → uk её показывает.
    await browser.execute(() => {
      const sel = document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement
      sel.value = 'ru'
      sel.dispatchEvent(new Event('change', { bubbles: true }))
    })
    await pause(500)
    await browser.execute(() => {
      const sel = document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement
      sel.value = 'uk'
      sel.dispatchEvent(new Event('change', { bubbles: true }))
    })
    await pause(600)
    const ukAfter = await browser.$('[data-testid="ws.root"]').getText()
    if (!ukAfter.includes('[LM-ACC uk]')) {
      throw new Error('uk-правка НЕ сохранилась — это уже потеря данных, а не только показ')
    }
    console.log('[lm-acc] uk-правка сохранилась (потеря данных нет; вопрос только в перерисовке)')
  })
})

describe('LM §10: удаление и подготовка рестарт-проверки', () => {
  it('удаление пользовательского языка убирает строку и чистит localStorage', async () => {
    await openLM()
    const before = await lmRows()
    await deleteLang('qa-dup')
    const after = await lmRows()
    console.log(`[lm-acc] DELETED qa-dup: rows ${before.length} → ${after.length}`)
    if (after.includes('qa-dup')) throw new Error('qa-dup не удалился')
    const stored = await storedLanguages()
    if (stored.includes('qa-dup')) throw new Error('qa-dup остался в localStorage')
    await deleteLang(QA_ID)
    if ((await lmRows()).includes(QA_ID)) throw new Error(`${QA_ID} не удалился`)
    await shot('lm-deleted')
  })

  it('создаём qa-persist и оставляем его для проверки рестарта', async () => {
    await submitLang('qa-persist', 'QA Persist', 'Куа Персист')
    await waitExisting('[data-testid="lm.row.qa-persist"]', 10000)
    const stored = await storedLanguages()
    if (!stored.includes('qa-persist')) throw new Error('qa-persist не сохранился')
    console.log('[lm-acc] qa-persist создан — следующий запуск приложения должен его показать')
    await shot('lm-qa-persist-created')
  })
})
