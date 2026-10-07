// D5: каталог-инпуты переживают ремоунт экрана. Экраны App монтируются на
// каждый роут, и useState('') стирал выбранный каталог при уходе/возврате.
// Значение живёт в sessionStorage под `rimloc.input.<scope>.<key>` и
// гидратируется при монте; scope неймспейсит контекст (project_id — по
// проекту, имя экрана — для беспроектных экранов вроде Compare).
// Приватный режим/недоступный storage — значение просто не переживает
// ремоунт, ничего не ломается.
import { useEffect, useState } from 'react'

const PREFIX = 'rimloc.input.'

export function usePersistedInput(scope: string, key: string): [string, (v: string) => void] {
  const storageKey = `${PREFIX}${scope}.${key}`
  const [value, setValue] = useState<string>(() => {
    try {
      return sessionStorage.getItem(storageKey) ?? ''
    } catch {
      return ''
    }
  })
  // Гидратация при смене scope (снапшот проекта пришёл после монта или
  // переключился сам проект): у другого проекта другое сохранённое значение,
  // а отсутствие записи — честная пустая строка.
  useEffect(() => {
    try {
      setValue(sessionStorage.getItem(storageKey) ?? '')
    } catch {
      /* хранилище недоступно — держим текущее значение */
    }
  }, [storageKey])
  const set = (v: string): void => {
    setValue(v)
    try {
      sessionStorage.setItem(storageKey, v)
    } catch {
      /* приватный режим — значение живёт только в состоянии компонента */
    }
  }
  return [value, set]
}
