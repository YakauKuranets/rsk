export const createFtpSlice = (set) => ({
  ftpBrowserOpen: false,
  activeServerAlias: import.meta.env.VITE_DEFAULT_SERVER_ALIAS || 'server1',
  ftpPath: '/',
  ftpItems: [],

  setFtpBrowserOpen: (v) => set({ ftpBrowserOpen: v }),
  setActiveServerAlias: (v) => set({ activeServerAlias: v }),
  setFtpPath: (v) => set({ ftpPath: v }),
  setFtpItems: (v) => set({ ftpItems: v }),
});
