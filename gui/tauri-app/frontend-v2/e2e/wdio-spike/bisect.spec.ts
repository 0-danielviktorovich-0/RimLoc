describe('bisect: service vs manual spawn', () => {
  it('backend op with manually-spawned app', async () => {
    const r = await browser.tauri.execute(async (tauri) => {
      const list = await tauri.core.invoke('project_list')
      return Array.isArray(list) ? { count: list.length } : { raw: 'x' }
    })
    console.log('[bisect] backend op:', JSON.stringify(r))
  })
})
