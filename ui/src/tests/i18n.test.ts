/**
 * i18n route keys test.
 *
 * Verifies that the expanded route keys in both locale files
 * resolve correctly through the i18n system.
 * Spec: FR-007 — i18n expansion with route keys.
 */
import { describe, it, expect } from 'vitest';

import en from '../i18n/locales/en.json';
import es from '../i18n/locales/es.json';

describe('i18n route keys', () => {
  it('en.json has all required route keys', () => {
    expect(en.routes).toBeDefined();
    expect(en.routes.home).toBe('Home');
    expect(en.routes.search).toBe('Search');
    expect(en.routes.playlists).toBe('Lists');
    expect(en.routes.now_playing).toBe('Now Playing');
    expect(en.routes.library).toBe('Library');
    expect(en.routes.settings).toBe('Settings');
  });

  it('es.json has all required route keys', () => {
    expect(es.routes).toBeDefined();
    expect(es.routes.home).toBe('Inicio');
    expect(es.routes.search).toBe('Buscar');
    expect(es.routes.playlists).toBe('Listas');
    expect(es.routes.now_playing).toBe('Reproduciendo');
    expect(es.routes.library).toBe('Biblioteca');
    expect(es.routes.settings).toBe('Configuración');
  });

  it('both locales have the same route key structure', () => {
    const enKeys = Object.keys(en.routes).sort();
    const esKeys = Object.keys(es.routes).sort();
    expect(enKeys).toEqual(esKeys);
  });

  it('common keys exist in both locales', () => {
    expect(en.common).toBeDefined();
    expect(es.common).toBeDefined();
    expect(en.common.loading).toBe('Loading...');
    expect(es.common.loading).toBe('Cargando...');
  });
});