// Seed demo PYMZA. Uso: mongosh < scripts/seed.js
// (idempotente: borra y reinserta las colecciones de demo)
const db = db.getSiblingDB('pymza');

// Guard: el seed hace dropDatabase(), solo debe correr contra la DB LOCAL.
const uri = (db.getMongo()._uri || '').toLowerCase();
if (!uri.includes('127.0.0.1') && !uri.includes('localhost')) {
  throw new Error('❌ Seed SOLO contra la DB local (127.0.0.1:27017). No tocar producción/Atlas.');
}

const EMPRESA = 'Ferretería El Tornillo';
// Tenant key = correo (contrato ola 1): los docs de planes_pago/dashboard_stats
// guardan `empresa: <correo>`, no el nombre comercial.
const CORREO_DEMO = 'demo@pymza.mx';

// Hash precomputado de 'demo123' (argon2id, params por defecto: m=19456, t=2, p=1).
// mongosh no puede hashear: se genera offline y se documenta aquí.
// Regenerar solo si cambia la contraseña demo (p. ej. con el backend: Argon2::default().hash_password).
const HASH_DEMO123 = '$argon2id$v=19$m=19456,t=2,p=1$S5gl3Bbi6xPFEATdpoI9yA$agd71TtOEoh0wIkeeAsnmfFJEVQs7UZHTcfx9aGpkZk';

db.dropDatabase();

db.empresas.insertMany([
  { correo: CORREO_DEMO, password: HASH_DEMO123, nombre_empresa: EMPRESA },
]);

db.clientes.insertMany([
  {
    curp: 'RAMJ920215MDFMZR03',
    nombre_completo: 'Janeth Ramos Zamora',
    score: 720,
    nivel_riesgo: 'Bajo',
    historial_pagos: 'Puntual en 2 empresas de la red',
    direccion: 'Av. Juárez 123, Centro',
    telefono: '5551234567',
  },
  {
    curp: 'GAML930528HDFLNR05',
    nombre_completo: 'Gabriel Martínez López',
    score: 640,
    nivel_riesgo: 'Medio',
    historial_pagos: '1 retraso de 5 días',
    direccion: 'Calle 5 de Mayo 88',
    telefono: '5559876543',
  },
  {
    curp: 'GARV850710MCHLRN09',
    nombre_completo: 'Vanessa García Ruiz',
    score: 510,
    nivel_riesgo: 'Alto',
    historial_pagos: 'Sin historial en la red',
    direccion: 'Andador Las Flores 4',
    telefono: '5554443322',
  },
]);

db.planes_pago.insertMany([
  {
    empresa: CORREO_DEMO,
    cliente_curp: 'RAMJ920215MDFMZR03',
    producto: 'Taladro Bosch',
    monto_total: 3200.0,
    plazo_meses: 6,
    pago_mensual: 565.33,
    tasa_interes: 0.06,
    estado: 'Activo',
    fecha: '2026-07-22',
  },
]);

db.dashboard_stats.insertOne({
  empresa: CORREO_DEMO,
  creditos_activos: 1,
  capital_prestado: 3200.0,
  proximos_cobros: 6,
});

print(`Seed OK — empresa demo: ${EMPRESA} (demo@pymza.mx / demo123)`);
