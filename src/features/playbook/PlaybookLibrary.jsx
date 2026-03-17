import React from 'react';

export const PLAYBOOK_PRESETS = [
  {
    name: 'Быстрое сканирование подсети',
    yaml: `name: "Quick Subnet Scan"
description: "Обнаружение камер в подсети и сбор метаданных"
scope:
  targets: ["\${TARGET_SUBNET}"]
  max_mode: "passive"
variables:
  TARGET_SUBNET: "192.168.1.0/24"
steps:
  - id: discover
    name: "Обнаружение камер"
    module: camera_scan
    params:
      target_input: "\${TARGET_SUBNET}"
      scan_mode: "fast"
  - id: metadata
    name: "Сбор метаданных"
    module: metadata_collect
    use_output_from: discover
    params:
      ip: "\${discover.cameras[0].ip}"`,
  },
  {
    name: 'Полный аудит Hikvision',
    yaml: `name: "Hikvision Full Audit"
description: "Полный аудит камеры Hikvision: креды, уязвимости, архив"
scope:
  targets: ["\${TARGET_IP}"]
  max_mode: "safe_active"
variables:
  TARGET_IP: "192.168.1.100"
steps:
  - id: scan
    name: "Сканирование портов"
    module: port_scan
    params:
      host: "\${TARGET_IP}"
  - id: creds
    name: "Проверка учётных данных"
    module: credential_audit
    requires_approval: true
    params:
      ip: "\${TARGET_IP}"
      vendor: "hikvision"
  - id: vulns
    name: "Проверка уязвимостей"
    module: vuln_scan
    params:
      ip: "\${TARGET_IP}"
      vendor: "hikvision"
  - id: archive
    name: "Поиск архивных записей"
    module: archive_search
    if: "creds.status == completed"
    params:
      camera_ip: "\${TARGET_IP}"`,
  },
  {
    name: 'Compliance-аудит',
    yaml: `name: "Compliance Check"
description: "Проверка соответствия стандартам PCI DSS и ISO 27001"
scope:
  targets: ["\${TARGET_SUBNET}"]
  max_mode: "passive"
variables:
  TARGET_SUBNET: "10.0.0.0/24"
steps:
  - id: discover
    name: "Обнаружение устройств"
    module: camera_scan
    params:
      target_input: "\${TARGET_SUBNET}"
      scan_mode: "normal"
  - id: headers
    name: "Аудит HTTP-заголовков"
    module: security_headers
    use_output_from: discover
    params:
      target_url: "http://\${discover.cameras[0].ip}"
  - id: compliance
    name: "Проверка соответствия"
    module: compliance_check
    params:
      standards: ["PCI_DSS", "ISO_27001"]`,
  },
];

export default function PlaybookLibrary({ onSelect }) {
  return (
    <div style={{ border: '1px solid #222', padding: 10, marginBottom: 10 }}>
      <h4 style={{ margin: '0 0 8px', color: '#7dff9c' }}>Playbook Library</h4>
      {PLAYBOOK_PRESETS.map((p) => (
        <button key={p.name} onClick={() => onSelect?.(p.yaml)} style={{ display: 'block', width: '100%', marginBottom: 6, background: '#111', color: '#00f0ff', border: '1px solid #333', padding: 6 }}>
          {p.name}
        </button>
      ))}
    </div>
  );
}
