import React, { useEffect } from 'react';
import { MapContainer, TileLayer, Marker, Popup, useMap } from 'react-leaflet';
import L from 'leaflet';
import icon from 'leaflet/dist/images/marker-icon.png';
import iconShadow from 'leaflet/dist/images/marker-shadow.png';

const DefaultIcon = L.icon({ iconUrl: icon, shadowUrl: iconShadow, iconSize: [25, 41], iconAnchor: [12, 41] });
L.Marker.prototype.options.icon = DefaultIcon;

function MapController({ center }) {
  const map = useMap();
  useEffect(() => {
    if (center) {
      map.setView(center, map.getZoom());
    }
  }, [center, map]);
  return null;
}

export default function StreamGrid({
  mapCenter,
  groupedMapTargets,
  fetchFtpRoot,
  setNemesisTarget,
  handleLocalArchive,
  handleFetchNvrDeviceInfo,
  handleFetchOnvifDeviceInfo,
  onCameraPlayClick,
}) {
  return (
    <MapContainer
      center={mapCenter}
      zoom={13}
      style={{ height: '100%', width: '100%' }}
      zoomControl
    >
      <MapController center={mapCenter} />
      <TileLayer url="https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png" />

      {Array.from(groupedMapTargets.values()).map((site) => (
        <Marker key={site.id} position={[site.lat, site.lng]}>
          <Popup>
            <div style={{ color: '#000', minWidth: '170px' }}>
              <strong>{site.siteName}</strong>
              <br />
              <div style={{ marginTop: '6px', marginBottom: '6px', color: '#444', fontSize: '11px' }}>
                Терминалов: {site.terminals.length}
              </div>
              <div style={{ marginTop: '8px', maxHeight: '300px', overflowY: 'auto' }}>
                {site.terminals.map((t) => (
                  <div key={t.id} style={{ borderTop: '1px solid #ddd', paddingTop: '8px', marginTop: '8px' }}>
                    <div style={{ fontWeight: 700, fontSize: '12px' }}>{t.name}</div>
                    <div style={{ color: '#666', fontSize: '10px', marginBottom: '6px' }}>{t.host}</div>

                    {t.channels?.map((ch) => (
                      <button
                        key={ch.id}
                        onClick={() =>
                          onCameraPlayClick?.({
                            ip: t.host,
                            terminalId: t.id,
                            terminal: t,
                            channel: ch,
                          })
                        }
                        style={{
                          display: 'block',
                          width: '100%',
                          marginBottom: '4px',
                          padding: '6px',
                          cursor: 'pointer',
                          backgroundColor: '#111',
                          color: '#00f0ff',
                          border: '1px solid #00f0ff',
                          fontSize: '11px',
                        }}
                      >
                        ▶ ОТКРЫТЬ: {ch.name}
                      </button>
                    ))}

                    {t.type === 'hub' ? (
                      <button
                        onClick={() => fetchFtpRoot('video1')}
                        style={{ display: 'block', width: '100%', marginTop: '8px', padding: '6px', cursor: 'pointer', backgroundColor: '#1a4a4a', color: '#00f0ff', border: '1px solid #00f0ff', fontSize: '11px', fontWeight: 'bold' }}
                      >
                        📁 АРХИВ ХАБА (FTP)
                      </button>
                    ) : (
                      <>
                        <button onClick={() => setNemesisTarget({ host: t.host, login: t.login || 'admin', password: t.password || '', name: t.name, channels: t.channels })} style={{ display: 'block', width: '100%', marginTop: '8px', padding: '6px', cursor: 'pointer', background: 'linear-gradient(90deg, #2a0808, #0a0808)', color: '#ff003c', border: '1px solid #ff003c', fontSize: '11px', fontWeight: 'bold', letterSpacing: '1px' }}>
                          ☢ NEMESIS ARCHIVE
                        </button>
                        <button onClick={() => handleLocalArchive(t)} style={{ display: 'block', width: '100%', marginTop: '6px', padding: '6px', cursor: 'pointer', backgroundColor: '#4a1a1a', color: '#ff9900', border: '1px solid #ff9900', fontSize: '11px', fontWeight: 'bold' }}>
                          ⏳ ЗАПРОС ПАМЯТИ
                        </button>
                        <button onClick={() => handleFetchNvrDeviceInfo(t)} style={{ display: 'block', width: '100%', marginTop: '6px', padding: '6px', cursor: 'pointer', backgroundColor: '#1a1a4a', color: '#9fc2ff', border: '1px solid #6a88ff', fontSize: '11px', fontWeight: 'bold' }}>
                          ℹ ISAPI DEVICE INFO
                        </button>
                        <button onClick={() => handleFetchOnvifDeviceInfo(t)} style={{ display: 'block', width: '100%', marginTop: '6px', padding: '6px', cursor: 'pointer', backgroundColor: '#1a3a1a', color: '#a8ffb0', border: '1px solid #47c45a', fontSize: '11px', fontWeight: 'bold' }}>
                          ℹ ONVIF DEVICE INFO
                        </button>
                      </>
                    )}
                  </div>
                ))}
              </div>
            </div>
          </Popup>
        </Marker>
      ))}
    </MapContainer>
  );
}
