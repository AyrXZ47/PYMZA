// Reparación de planes legacy con dinero inconsistente (ola 8-fix).
//
// Detecta:
//   1. `pago_mensual` que no coincide con `pago_mensual_de(monto_total, plazo)`
//      (p. ej. fabricado por el bug E2: `pago_mensual: Infinity`), y
//   2. `cobrado` mayor que la suma del ledger (`pagos`) — el contador quedó
//      inflado por una reserva sin su pago.
//
// Con la bandera de aplicación:
//   - recomputa `tasa_interes` y `pago_mensual` desde `monto_total` + `plazo`
//     con la tabla vigente (1m 7%, 3m 9%, 6m 12%, 9m 15%, 12m 18%), y
//   - fija `cobrado` = suma del ledger.
//
// USO (dry-run por defecto; nunca escribe):
//   mongosh backend/scripts/reparar_planes.js
// Para aplicar:
//   APPLY=1 mongosh backend/scripts/reparar_planes.js
//   # o: mongosh --eval "var APPLY=true" --file backend/scripts/reparar_planes.js
// (mongosh rechaza flags propios como `--apply`; la bandera va por env/global.)
//
// NO se corre en el humo: lo ejecuta V bajo su control (backup/Atlas). Es
// idempotente: un plan ya consistente no vuelve a aparecer.
const db = db.getSiblingDB('pymza');

const APLICAR =
  (typeof APPLY !== 'undefined' && APPLY === true) ||
  process.env.APPLY === '1' ||
  process.env.APPLY === 'true';

const TASAS = { 1: 0.07, 3: 0.09, 6: 0.12, 9: 0.15, 12: 0.18 };
const redondear2 = (x) => Math.round(x * 100) / 100;
// Misma fórmula del backend (`pago_mensual_de`). `null` si el plazo no está en
// la tabla vigente (plan corrupto de otra forma: no se puede recomputar).
const pagoMensualDe = (monto, plazo) => {
  const tasa = TASAS[plazo];
  if (tasa === undefined || !isFinite(monto)) return null;
  return redondear2((monto * (1 + tasa)) / plazo);
};

// Ledger por plan: suma de todos los pagos/abonos.
const totalPorPlan = {};
db.pagos
  .aggregate([{ $group: { _id: '$plan_id', total: { $sum: '$monto' } } }])
  .forEach((d) => {
    totalPorPlan[String(d._id)] = d.total;
  });

let revisados = 0;
let porReparar = 0;

db.planes_pago.find({}).forEach((plan) => {
  revisados++;
  const id = String(plan._id);
  const ledger = totalPorPlan[id] || 0;
  const esperado = pagoMensualDe(plan.monto_total, plan.plazo_meses);
  const pagoActual = plan.pago_mensual || 0;
  // Compara el pago mensual guardado contra el recalculado (tolerancia 1 centavo).
  const pagoMalo = esperado === null || Math.abs(pagoActual - esperado) > 0.01;
  const cobradoMalo = (plan.cobrado || 0) > ledger + 1e-9;

  if (!pagoMalo && !cobradoMalo) return;

  porReparar++;
  if (esperado === null) {
    print(`⚠️  plan ${id}: plazo ${plan.plazo_meses} fuera de la tabla vigente — se omite pago_mensual`);
  }
  const cambios = [];
  if (pagoMalo && esperado !== null) {
    cambios.push(`pago_mensual ${pagoActual} → ${esperado} (tasa ${plan.tasa_interes} → ${TASAS[plan.plazo_meses]})`);
  }
  if (cobradoMalo) {
    cambios.push(`cobrado ${plan.cobrado} → ${ledger}`);
  }
  if (cambios.length === 0) {
    print(`🔎 plan ${id} (${plan.empresa}): sin cambios aplicables`);
    return;
  }
  print(`${APLICAR ? '🔧' : '🔎'} plan ${id} (${plan.empresa}, ${plan.cliente_curp}): ${cambios.join('; ')}`);

  if (APLICAR) {
    const set = {};
    if (pagoMalo && esperado !== null) {
      set.pago_mensual = esperado;
      set.tasa_interes = TASAS[plan.plazo_meses];
    }
    if (cobradoMalo) {
      set.cobrado = ledger;
    }
    db.planes_pago.updateOne({ _id: plan._id }, { $set: set });
  }
});

print(
  `${APLICAR ? '✅ Reparación aplicada' : '🔎 Dry-run'}: ${revisados} planes revisados, ` +
    `${porReparar} ${APLICAR ? 'reparados' : 'por reparar'}.`
);
if (!APLICAR && porReparar > 0) {
  print('Vuelve a correr con APPLY=1 para escribir los cambios.');
}
