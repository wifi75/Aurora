import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createRoot } from 'react-dom/client';
import './styles/tokens.css';
import './styles/base.css';
import './styles/components.css';
import './styles/glass.css';
import './styles/onair.css';
import './styles/bozze.css';
import { FreqPicker } from './components/FreqPicker';
import { ModeSwitch } from './components/ModeSwitch';
import { BandBar } from './components/BandBar';

const rows = [
  ['40m', 7_047_500, true], ['40m', 7_056_000, false], ['40m', 7_071_000, false], ['40m', 7_074_000, true], ['40m', 7_080_000, false],
  ['30m', 10_131_000, false], ['30m', 10_136_000, true], ['20m', 14_071_000, false], ['20m', 14_074_000, true], ['20m', 14_090_000, false],
  ['160m', 1_810_000, true, 'R1'], ['160m', 1_840_000, true], ['10m', 28_074_000, true], ['10m', 28_091_000, false], ['6m', 50_313_000, true], ['6m', 50_323_000, true],
].map(([band, hz, preferred, region]) => ({ band, mode: 'FT8', hz, preferred, region: region ?? 'ALL' }));
const qc = new QueryClient();
qc.setQueryData(['frequencies'], rows);
createRoot(document.getElementById('root')!).render(
  <QueryClientProvider client={qc}>
    <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
      <ModeSwitch mode="FT8" onPick={() => {}} />
      <BandBar dialHz={28_074_000} bands={['40m', '20m', '15m', '10m', '6m']} onPick={() => {}} />
      <FreqPicker dialHz={28_074_000} mode="FT8" onPick={() => {}} />
    </div>
  </QueryClientProvider>,
);
