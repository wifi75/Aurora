import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { useRadioDisplay } from '../../lib/radioDisplay';
import { useStation } from '../../state/station';
import { alcWarn, powerCeiling, swrFraction, TxMetersBig } from './TxMetersBig';

describe('pannello grande di potenza, ROS e ALC', () => {
  beforeEach(() => {
    useRadioDisplay.setState({ pref: { style: 'fluorescent', hidden: false, layout: 'single', bigMeters: true } });
  });

  it('la scala della potenza e della barra del ROS', () => {
    expect(powerCeiling(7)).toBe(10);
    expect(powerCeiling(60)).toBe(100);
    expect(powerCeiling(5000)).toBe(1500);
    expect(swrFraction(1)).toBe(0);
    expect(swrFraction(2)).toBe(0.5);
    expect(swrFraction(9)).toBe(1);
    expect(alcWarn(0.7)).toBe(true);
    expect(alcWarn(0.3)).toBe(false);
  });

  it('senza strumenti dalla radio non compare', () => {
    useStation.setState({ status: null } as never);
    const { container } = render(<TxMetersBig />);
    expect(container).toBeEmptyDOMElement();
  });

  it('spento dalle Impostazioni non compare', () => {
    useRadioDisplay.setState({ pref: { style: 'fluorescent', hidden: false, layout: 'single', bigMeters: false } });
    const { container } = render(<TxMetersBig />);
    expect(container).toBeEmptyDOMElement();
    expect(screen.queryByRole('group')).toBeNull();
  });
});
