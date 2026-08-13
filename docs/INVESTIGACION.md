# INVESTIGACIÓN: Integraciones para PYMZA (buró, FICO, Open Banking, Stripe)

> Documento de investigación (T-18) — alimenta las decisiones de T-26 (pagos/cobranza) y T-30+ (perfilación crediticia).
> Fecha: 2026-08-11 · Solo fuentes públicas citadas al final de cada sección. Cero keys, cero datos reales.
> Modelo: rápido · Riesgo: nulo · Solo documentación.

Resumen ejecutivo:

| Integración | Esfuerzo | Costo típico | Estado sugerido |
|---|---|---|---|
| Círculo de Crédito (buró) | M | Por consulta, no público (contrato) | Postergar a T-30+; sandbox ya accesible |
| FICO® Score México | S (suma al buró) | Incluido en consulta buró | Igual que buró |
| Open Banking (Belvo/Finerio) | M | Belvo desde USD 1,000/mes; Finerio cotiza | Postergar; sandbox gratuito para validar |
| Stripe MX (pagos + Billing) | S | 3.6% + MXN 3.00 por transacción; Billing 0.7% | Prioridad alta para T-26 |

---

## 1. Círculo de Crédito / buró nacional

**Qué es.** Círculo de Crédito (CdC) es una Sociedad de Información Crediticia (SIC) regulada por CNBV, Banxico y CONDUSEF; el otro buró de México es Buró de Crédito S.A. (productos equivalentes). Ambos concentran el historial crediticio de personas físicas y morales: cuentas, saldos, morosidad, consultas y scores. Nota: en 2026 Equifax firmó acuerdo definitivo para adquirir Círculo de Crédito (fuente: comunicado en empresas.circulodecredito.com.mx) — vigilar posibles cambios de catálogo/contrato.

Para PYMZA el camino es el **API Hub de desarrolladores de CdC** (`developer.circulodecredito.com.mx`), que expone catálogo amplio:

- Reportes de crédito: RCC Personas Físicas (v1/v2), RCC Personas Morales, RCE, Reporte de Crédito + FICO® Score.
- Scores: FICO® Score, Extended Score, Financial Inclusion Score (v1/v1.1/v2), FinTech Score, Loan Amount Estimator.
- Verificación y fraude: Employment Verification, SAT Personas Morales, PLD Check (personas físicas/morales), Mobile Identity, Bank Account Verification, Address Verification.
- Open Banking para Personas Morales (saldos y movimientos de cuentas bancarias).

**Costo estimado.** Las tarifas **no son públicas**: se cotizan por consulta (reporte/score) bajo contrato de afiliación con ejecutivo comercial. El alta como otorgante de crédito requiere razón social y contrato con CdC (el formulario de pase a producción pide "Número de otorgante", "Razón social" y el nombre del ejecutivo comercial). Referencia de mercado: las consultas de buró para otorgantes se facturan en pesos por transacción con descuentos por volumen; no hay mensualidad fija publicada.

**Esfuerzo de integración: M.**
- Sandbox **gratuito y sin contrato**: crear cuenta de desarrollador en el API Hub y registrar una app (5 min); las APIs de simulación responden con datos ficticios.
- Producción: afiliación como otorgante + firma electrónica de cada petición con par de llaves ECDSA (`secp384r1`, generado con OpenSSL) y verificación de la firma de respuesta (cabecera `x-signature`), Security Test, y pase a producción (aprobación reportada hasta ~3 días).
- Hay clientes de referencia en GitHub (`APIHub-CdC/*-client-java`, `-php`) y Swagger/Postman por API; en Rust no hay SDK oficial → habría que implementar la firma con una crate criptográfica.

**Requisitos clave.** 1) Contrato/afiliación como otorgante para producción. 2) Cumplir con la normativa de protección de datos (las SIC exigen fines de otorgamiento de crédito). 3) Llave privada del cliente guardada como secreto (jamás en repo; regla PYMZA).

**Recomendación.** Crear hoy la cuenta de desarrollador y validar el sandbox (gratis, sin trámites) para dimensionar el producto. Dejar la contratación de producción para T-30+, cuando exista flujo real de solicitudes de crédito y la entidad jurídica necesaria para afiliación. Alternativa de menor fricción si solo se necesita score: ver §2.

**Fuentes.** empresas.circulodecredito.com.mx (productos, regulación, ISO 27001, comunicado Equifax); developer.circulodecredito.com.mx/guia_de_inicio (pasos sandbox→producción, claves, firma); developer.circulodecredito.com.mx/apis (catálogo); /pase_a_produccion (requisitos).

---

## 2. FICO® Score México

**Qué es.** El FICO® Score México es un score crediticio de FICO Inc. (escala 300–850; a mayor puntaje, menor riesgo de incumplimiento) que se distribuye en México **a través de las dos SIC**: Buró de Crédito y Círculo de Crédito. No se contrata con FICO directamente.

**Cómo se consume.** Vía API de buró:
- CdC API Hub: API "FICO® Score" (solo el puntaje) y "Reporte de Crédito + FICO® Score Personas Físicas" (reporte completo + score en una consulta). Ambas con modo simulación y producción.
- Buró de Crédito ofrece su equivalente propietario "Mi Score" (puntaje propio, dinámico) y productos FICO® Score para otorgantes; su centro de ayuda confirma que el score requiere historial crediticio con al menos ~6 meses de antigüedad y que se genera a partir del reporte (no se puede consultar score sin consultar historial).

**Costo estimado.** No público. Se factura como parte de la consulta de buró (precio por consulta bajo contrato; una consulta "reporte + FICO" cuesta lo que el reporte más un incremento por score, según tarifa del otorgante).

**Esfuerzo de integración: S** (adicional al buró). Si ya se consume RCC vía CdC, añadir el score es activar una API más del mismo hub con la misma firma/credenciales; el único trabajo extra es mapear el campo de score a `clientes.score` de PYMZA (hoy se asigna score base 550 al alta de cliente).

**Recomendación.** Adoptar como parte de la integración de buró (T-30+), no por separado: el reporte solo es útil con score, y la consulta combinada cuesta un solo round-trip. Advertencia: clientes sin historial (comunes en PYMES) no obtienen FICO; para ellos seguir dependiendo del score interno de PYMZA o de open banking (flujo de caja real).

**Fuentes.** developer.circulodecredito.com.mx/producto/fico-score (API, sandbox, clientes Java/PHP); developer.circulodecredito.com.mx/apis (RCC+FICO Score); burodecredito.com.mx/generales/centro-de-ayuda/mi-score.html (requisito de historial, puntaje dinámico); fico.com (documentación pública del score para México).

---

## 3. Open Banking — Belvo y Finerio

**Qué es.** Agregadores de open finance que conectan la cuenta bancaria del cliente (con su consentimiento) y exponen saldos, movimientos, categorización y, en algunos casos, cobranza. Útiles para PYMZA para: verificar flujo de caja real (mejor score que el buró para PYMES sin historial) y cobranza recurrente por domiciliación.

### Belvo
- **Qué expone (México, según su portal de desarrolladores al 2026):** Employment data (registros oficiales de empleo/IMSS), Fiscal data (CFDI/facturas SAT), Payments / Direct Debit (cobranza recurrente en México). Nota importante: el portal de desarrolladores ya **no lista agregación bancaria ("Banking data") para México** (solo Brasil); en México el producto vigente es empleo + fiscal + pagos. La página principal sigue publicitando Banking data genérico — verificar disponibilidad MX con ventas antes de diseñar.
- **Costo:** plan Launch **USD 1,000/mes** (primer plan de pago publicado); Growth custom; **sandbox gratuito** para pruebas ilimitadas.
- **Esfuerzo:** M. API REST documentada, widgets de conexión (Connect Widget/Page), SDKs; sandbox inmediato. En Rust no hay SDK oficial → cliente HTTP propio (el stack PYMZA ya usa reqwest en el backend).
- **Recomendación:** el sandbox sirve hoy para validar el flujo de "verificar ingresos con CFDI/IMSS". La suscripción de USD 1,000/mes no se justifica en etapa temprana; reconsiderar en T-30+ con volumen.

### Finerio Connect
- **Qué es.** Financiera de open finance mexicana (primera de Hispanoamérica; +120 clientes en LATAM, 98% tasa de conexión, ISO 27001). Enfocada en agregación bancaria (saldos y transacciones), categorización de datos y "Open Finance in a Box" (APIs propias para cumplimiento regulatorio).
- **Qué expone:** datos agregados de cuentas bancarias (retail y empresas) y categorización; caso de uso típico: risk assessment / underwriting en crédito.
- **Costo:** **no público**, cotización por volumen/uso (modelo B2B).
- **Esfuerzo:** M. API REST + widgets de conexión; mismo patrón que Belvo.
- **Recomendación:** alternativa real si se necesita agregación bancaria en México (Belvo la retiró de MX); pedir cotización solo cuando el caso de uso de flujo de caja esté validado con clientes.

**Fuentes.** belvo.com/plans-and-pricing (planes y precios); developers.belvo.com (catálogo MX: Employment, Fiscal, Payments); belvo.com (productos, ISO 27001, métricas públicas); finerio.mx (agregación, categorización, Open Finance in a Box, métricas, ISO 27001).

---

## 4. Stripe (pagos y cobranza recurrente para PYMES MX)

**Qué es.** Procesador de pagos global con presencia completa en México: tarjetas (crédito/débito/prepago), carteras digitales, OXXO, SPEI, cuotas (MSI) y suscripciones vía Stripe Billing. Cobra solo por transacción (sin mensualidad). Relevante para PYMZA en dos frentes: cobranza de créditos (MSI/SPEI/tarjeta) y pagos recurrentes de comisiones.

**Costos (tarifas públicas MX, excluyen IVA):**
- Tarjetas nacionales: **3.6% + MXN 3.00** por transacción; internacionales +0.5%; +2% si hay conversión de moneda.
- Cuotas/meses sin intereses: desde **5%** (3 meses) — ver tabla oficial por plazo.
- Métodos bancarios (SPEI, etc.) y otros locales: **4% + MXN 3.00**; OXXO disponible como método.
- **Billing** (suscripciones/cobranza recurrente): **0.7%** del volumen facturado.
- Disputas: MXN 150.00 cada una. Radar anti-fraude desde MXN 0.95/transacción. 3D Secure incluido.
- Sin costos de instalación ni mensualidades.

**Esfuerzo de integración: S.** Documentación y SDKs en todos los lenguajes (incluido Rust vía API REST o webhooks); Checkout preconstruido (sin UI propia) o Payment Links (cero código); keys de test vs. live con datos ficticios (modelo de tarjetas `4242...`). Webhooks para conciliar pagos con `planes_pago` de PYMZA.

**Alternativas accesibles para PYMES MX** (misma categoría, si Stripe no encaja): **Conekta** (fintech mexicana, tarjetas + OXXO + SPEI, fuerte en cobranza MX), **Mercado Pago** (tarjetas + efectivo + SPEI; popular en PM), **Clip** (TPV físico/presencial), y cobro por **CLABE/SPEI manual** (costo cero pero operación manual). Para cobranza domiciliada recurrente sin tarjeta, la alternativa es Belvo Direct Debit (§3).

**Recomendación.** **Prioridad alta para T-26**: es la única integración lista para producción con costo puramente transaccional, sin contrato ni mensualidad. Empezar con Checkout + tarjetas, sumar OXXO/SPEI y Billing (0.7%) para la cobranza recurrente. Dejar las alternativas locales (Conekta/Mercado Pago) solo si los clientes PYMES exigen canales que Stripe MX no cubra.

**Fuentes.** stripe.com/mx/pricing (tarifas MX completas: Payments, cuotas, métodos bancarios, Billing, Radar, disputas); stripe.com/mx/pricing/local-payment-methods (OXXO/SPEI); docs.stripe.com (SDKs, Checkout, webhooks).

---

## Implicaciones para T-26 y T-30+

- **T-26 (pagos/cobranza):** Stripe es el camino de menor fricción (esfuerzo S, sin contrato). Diseñar el módulo de cobranza con una interfaz de "proveedor de pagos" para no acoplarse a Stripe; los campos mínimos a persistir: id de pago, método, estado, webhook de confirmación. La cobranza domiciliada (tarjeta/SPEI recurrente) la cubre Stripe Billing; si se quisiera domiciliación bancaria directa, evaluar Belvo Direct Debit (repetir investigación antes de comprar el plan).
- **T-30+ (perfilación):** buró + FICO comparten el mismo contrato/afiliación y la misma API — tratarlos como un solo proyecto. El score de FICO mejora el `score` interno (550 base) solo para clientes con historial; para el resto, open banking (Finerio en MX) o datos fiscales (Belvo) son el complemento. Ninguna de estas integraciones toca el modelo de datos actual de PYMZA; el punto de integración natural es el endpoint `POST /api/creditos/evaluar` (entrada de score externo) — definir contrato ahí cuando se decida.
- Presupuesto de referencia: Stripe ≈ costo transaccional puro; Belvo ≈ USD 1,000/mes; buró ≈ por consulta (estimar con volumen real antes de firmar).
