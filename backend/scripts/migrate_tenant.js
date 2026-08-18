// Migración multi-tenant (ola 1): antes los documentos de `planes_pago` y
// `dashboard_stats` guardaban `empresa: <nombre_empresa>`; ahora el tenant key
// es `empresa: <correo>` (único y validado, ver decision log 2026-08-13).
//
// Idempotente: al correrlo N veces, la primera actualiza los documentos y las
// siguientes hacen match con 0 documentos (la segunda pasada coincidiría con el
// correo, no con el nombre). Puede correrse contra local o Atlas.
//
// Uso: mongosh < backend/scripts/migrate_tenant.js
const db = db.getSiblingDB('pymza');

let total = 0;
for (const empresa of db.empresas.find({}).toArray()) {
  const enPlanes = db.planes_pago.updateMany(
    { empresa: empresa.nombre_empresa },
    { $set: { empresa: empresa.correo } },
  );
  const enDashboard = db.dashboard_stats.updateMany(
    { empresa: empresa.nombre_empresa },
    { $set: { empresa: empresa.correo } },
  );
  total += enPlanes.modifiedCount + enDashboard.modifiedCount;
}

print(
  `Migración tenant OK — ${db.empresas.countDocuments()} empresas procesadas, ` +
    `${total} documentos actualizados a correo como tenant key`,
);