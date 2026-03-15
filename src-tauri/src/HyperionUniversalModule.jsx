import { invoke } from '@tauri-apps/api/core';

export const HyperionUniversalProtocol = {
    // Эта функция принимает IP, логин и пароль от пользователя
    engage: async function(targetIp, login, password, onProgressUpdate) {
        onProgressUpdate(`🚀 [HYPERION] Запуск универсального протокола для ${targetIp}...`);

        try {
            // ==========================================
            // ФАЗА 1: Порты
            // ==========================================
            onProgressUpdate("⏳ [1/4] Сканирование открытых портов...");
            const ports = await invoke("scan_host_ports", { host: targetIp });
            const openPorts = ports.filter(p => p.open).map(p => p.port);
            onProgressUpdate(`✅ Открытые порты: ${openPorts.length > 0 ? openPorts.join(", ") : "НЕТ"}`);

            if (openPorts.length === 0) {
                return { success: false, message: "Цель мертва. Открытых портов не найдено." };
            }

            // ==========================================
            // ФАЗА 2: Протоколы
            // ==========================================
            onProgressUpdate("⏳ [2/4] Идентификация систем (ISAPI/ONVIF)...");
            const protocols = await invoke("probe_nvr_protocols", { host: targetIp, login, pass: password });

            const hasIsapi = protocols.some(p => p.protocol === "ISAPI" && p.status === "detected");
            const hasOnvif = protocols.some(p => p.protocol === "ONVIF" && p.status === "detected");
            onProgressUpdate(`✅ Протоколы: ISAPI: ${hasIsapi ? "ДА" : "НЕТ"}, ONVIF: ${hasOnvif ? "ДА" : "НЕТ"}`);

            // ==========================================
            // ФАЗА 3: Паук (Криптография)
            // ==========================================
            let spiderReport = null;
            // Запускаем паука только если есть HTTP-порты
            if (openPorts.includes(80) || openPorts.includes(8080) || openPorts.includes(2019)) {
                onProgressUpdate("⏳ [3/4] Запуск Паука-Анализатора (вскрытие JS)...");
                const spiderUrl = openPorts.includes(80) ? `http://${targetIp}` : `http://${targetIp}:2019`;

                spiderReport = await invoke("spider_full_scan", {
                    targetUrl: spiderUrl,
                    cookie: null,
                    maxDepth: 2,
                    maxPages: 15,
                    dirBruteforce: false // Отключено для скорости
                });

                const cryptoCount = spiderReport.cryptoFormulas ? spiderReport.cryptoFormulas.length : 0;
                onProgressUpdate(`✅ Паук завершен. Найдено алгоритмов шифрования: ${cryptoCount}`);
            }

            // ==========================================
            // ФАЗА 4: Поиск Архивов
            // ==========================================
            onProgressUpdate("⏳ [4/4] Штурм архивов (поиск записей)...");
            let isapiArchives = [];
            let onvifArchives = [];

            if (hasIsapi) {
                try {
                    isapiArchives = await invoke("search_isapi_recordings", {
                        host: targetIp, login, pass: password,
                        fromTime: "2024-01-01T00:00:00Z", toTime: "2027-12-31T23:59:59Z"
                    });
                    onProgressUpdate(`🎯 Найдено ${isapiArchives.length} записей ISAPI.`);
                } catch (e) {
                    onProgressUpdate(`⚠️ Ошибка ISAPI: ${e}`);
                }
            }

            if (hasOnvif && isapiArchives.length === 0) {
                try {
                    onvifArchives = await invoke("search_onvif_recordings", { host: targetIp, login, pass: password });
                    onProgressUpdate(`🎯 Найдено ${onvifArchives.length} записей ONVIF.`);
                } catch (e) {
                    onProgressUpdate(`⚠️ Ошибка ONVIF: ${e}`);
                }
            }

            // ==========================================
            // ФИНАЛ
            // ==========================================
            onProgressUpdate("🏆 ПРОТОКОЛ УСПЕШНО ЗАВЕРШЕН!");

            return {
                success: true,
                targetIp,
                openPorts,
                capabilities: { isapi: hasIsapi, onvif: hasOnvif },
                spider: spiderReport,
                archives: { isapi: isapiArchives, onvif: onvifArchives }
            };

        } catch (error) {
            onProgressUpdate(`❌ КРИТИЧЕСКИЙ СБОЙ: ${error}`);
            return { success: false, message: error.toString() };
        }
    }
};