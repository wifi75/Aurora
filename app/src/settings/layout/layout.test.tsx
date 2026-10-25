import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { translate } from '../../i18n';
import { Cells, Field, Group, Notes, Page, Unavailable } from './index';

describe('settings layout', () => {
  it('a group has a visible title and its cells', () => {
    render(
      <Page>
        <Group title="Collegamento" span={2}>
          <Cells>
            <Field label="Porta">
              <input type="text" />
            </Field>
          </Cells>
        </Group>
      </Page>,
    );
    expect(screen.getByRole('heading', { name: 'Collegamento' })).toBeInTheDocument();
    expect(document.querySelector('.sx-span-2')).not.toBeNull();
  });

  it('a field is a label: the text names the control and a click on it acts on the control', () => {
    const change = vi.fn();
    render(
      <Field label="Usa il nominativo">
        <input type="checkbox" onChange={change} />
      </Field>,
    );
    fireEvent.click(screen.getByText('Usa il nominativo'));
    expect(change).toHaveBeenCalledTimes(1);
    expect(screen.getByLabelText('Usa il nominativo')).toBeInTheDocument();
  });

  it('keeps the whole text of a long label in the tooltip and marks a changed or stacked field', () => {
    render(
      <Field label="Un testo molto lungo che nella cella si ferma a due righe" changed stack>
        <textarea />
      </Field>,
    );
    const cell = screen.getByTitle('Un testo molto lungo che nella cella si ferma a due righe');
    expect(cell).toHaveClass('sx-field--changed', 'sx-field--stack');
  });

  it('notes and unavailable items are collapsible boxes with a translated title', () => {
    render(
      <Page>
        <Notes>
          <p>Una nota lunga</p>
        </Notes>
        <Unavailable count={3}>
          <p>Voce spenta</p>
        </Unavailable>
      </Page>,
    );
    expect(screen.getByText(translate('it', 'lay.notes'))).toBeInTheDocument();
    expect(screen.getByText(translate('it', 'ens.hidden', { n: 3 }))).toBeInTheDocument();
    expect(document.querySelectorAll('details')).toHaveLength(2);
    expect(screen.getByText('Una nota lunga')).toBeInTheDocument();
  });

  it('shows no box when nothing is unavailable', () => {
    render(
      <Unavailable count={0}>
        <p>niente</p>
      </Unavailable>,
    );
    expect(document.querySelector('details')).toBeNull();
  });
});
