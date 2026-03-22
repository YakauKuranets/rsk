export function computeLaunchReadiness({ intelligenceTarget, permit, args }) {
  const normalizedTarget = String(intelligenceTarget || '').trim();
  const normalizedPermit = String(permit || '').trim();
  const normalizedArgs = String(args || '').trim();
  const hasTemplatePlaceholders = /(^|\b)(TARGET|FUZZ|example\.com)(\b|$)/i.test(normalizedArgs);

  if (!normalizedTarget) return { level: 'error', text: 'Нужно указать цель', canRun: false };
  if (normalizedPermit.length < 8) return { level: 'error', text: 'Нужен токен', canRun: false };
  if (!normalizedArgs) return { level: 'error', text: 'Проверь аргументы', canRun: false };
  if (hasTemplatePlaceholders) return { level: 'warn', text: 'Проверь аргументы', canRun: true };
  return { level: 'ok', text: 'Готово к запуску', canRun: true };
}
