import { create } from '../../vendor/zustand/index.js';
import { createSpiderSlice } from './slices/spiderSlice';
import { createFtpSlice } from './slices/ftpSlice';
import { createFuzzSlice } from './slices/fuzzSlice';
import { createMassAuditSlice } from './slices/massAuditSlice';
import { createIntelligenceSlice } from './slices/intelligenceSlice';

export { SPIDER_MODULES_CONFIG } from './slices/spiderSlice';

export const useAppStore = create((...a) => ({
  ...createSpiderSlice(...a),
  ...createFtpSlice(...a),
  ...createFuzzSlice(...a),
  ...createMassAuditSlice(...a),
  ...createIntelligenceSlice(...a),

  hubCookie: '',
  setHubCookie: (v) => a[0]({ hubCookie: v }),
}));
