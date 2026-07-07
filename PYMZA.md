---
tags:
  - proyecto/idea
  - dev
fecha: 2026-06-06
estado: 🟢 Activo
---
# 🚀 PYMZA

## 🎯 El Concepto (Elevator Pitch)
> App para perfilación de crédito y cobranza dirigido a empresas para PYMES. Página web (Yo digo que mejor una app en tauri)

---


## 🛠️ Stack Tecnológico
*   **Frontend / UI:** (Ej. Obsidian Canvas API)
*   **Backend / Lógica:** Rust, mongo db.
*   **Infraestructura:** Railway (probablemente, pero ojalá que se distribuya en tiendas de apps)
*   **Integraciones:** por definir.

---

## 🗺️ Hitos y Tareas (Roadmap)
- [x] investigar api de circulo de credito o buró de crédito.
- [x] investigar plataformas de Open Banking (como Belvo o Finerio)
- [x] system design 
- [x] validación por ocr

---

## 🧠 Brainstorming y Notas Sueltas (Idea original, primeros pasos)
*   Las empresas crean un perfil financiero ligado a la ine de los solicitantes, en caso de ser nuevo perfil y basado en históricos brindar a la empresa niveles de confianza del solicitante de crédito. En caso de ya haberse dado de alta con otra empresa por otro producto a crédito analizar su perfil y su nivel de confianza, si resulta ser cliente moroso y además se encuentra en calidad de desaparecido la empresa activa una pequeña alerta o flag en el software que anuncia que fue visto en ese local. 
* El Motor de Datos Alternativos: El cliente típico de muchas PYMES a veces opera en la informalidad o no tiene tarjetas de crédito bancarias. En lugar de solo consultar un historial crediticio clásico, el backend podría perfilar el riesgo analizando otras fuentes. Por ejemplo, procesar comprobantes de ingresos o leer el historial de pagos de servicios básicos (luz, agua, telecomunicaciones). Si un cliente paga sus servicios impecablemente, el algoritmo le asigna un "Score de Confianza" alto, creando un modelo de riesgo alterno válido para el hackaton.
* La Red de Alerta Temprana (El "Sistema Inmunitario" de PYMES): Sistema de reputación colaborativo. Si un "Sujeto A" comete fraude o desaparece con una deuda en una PYME, ese nodo actualiza la base de datos. Cuando el sujeto intente sacar crédito en otro negocio de la red, el sistema arroja la alerta temprana. Esto crea una red de protección comunitaria que las herramientas corporativas no suelen tener. (blockchain? me sonó a un buen momento para analizar el uso de estas herramientas sobre la red de ethereum o algo así)
* ​Onboarding con Ciberseguridad: Integrar un microservicio rápido de validación por OCR para verificar que la identificación sea legítima y evitar la suplantación de identidad. Esto ataca directamente el rubro de "Seguridad informática y prevención de fraude".

# ==Nueva estructura derivada de la arquitectura de software (Producto real: SaaS B2B Multi-Tenant)==

***Cómo funciona eso de que entras al sistema y se precargan los datos de cualquier cliente como si nada?***

> Las empresas no pueden estar teniendo acceso a toda la red de clientes que ni siquiera les pertenencen, hay datos que son relevantes para mí como empresa PYMZA, mis datos, mis metricas, aquí si puedo ver a los clientes y empresas, etc. Y hay datos que puede ver solo cada empresa, digamos, la tiendita de la esquina y la ferretería de enfrente les vendí mi suscripción anual o mensual a mi software para que puedan ofrecer productos a crédito de manera estructurada y fiable a sus clientes, aunque primero deben ver si los clientes ya están dados de alta a través de otra empresa o no para no tener duplicados de perfiles en el servidor y poder reutilizar su perfil, eso los clientes finales no lo ven, si no tienen perfil se les crea uno ligado a su ine, recibos, etc, si ya tienen un perfil entonces se reutiliza y ahora los datos y su nivel de confianza se ligan tomando en cuenta los pagos a tiempo que hayan tenido con las otras empresas en las que ya se habían dado de alta dentro de nuestro mismo software. 

1. El frontend de empresas finales es diferente al frontend de servicio técnico de PYMZA (el servicio tecnico que tendremos al atender a clientes molestos con fallos o cosas así, no se les dará acceso a la db real verdad??).
2. El frontend para inversionistas no es el mismo que el que tendrán los de servicio técnico o sí? la mesa directiva debe tener acceso a los datos, métricas, consumo, etc. 
3. El frontend que verá cada empresa final debe tener solo los datos relevantes ligados a su propia empresa, sus clientes, sus fechas de cobro el panel para dar de alta un nuevo cliente (que por cierto ese panel debe validar que ese cliente ya esté dado de alta a través de otra empresa de ser así en vez de crear todo un perfil y validar su score desde cero, dar de alta el perfil ya existente del cliente dentro de la interfaz de la empresa que lo solicita y validar si le aprueban cierto producto a crédito o no.)
4. Un panel en donde se vean los planes a pagos activos que tiene cada cliente que mantiene con la empresa final. 
5. Capacidad de poder dar de alta a empresas para que tengan acceso real a su cuenta con contraseña 

Lo correcto sería primero mostrar la interfaz que verá la empresa al iniciar su respectiva sesión, y luego tener un área o botón que diga dar de alta a nuevo cliente, comienza el proceso llenando los inputs y precargando archivos, le da click en siguente y la interfáz da una animación de que se desplaza a una nueva ventana para subir, llenar y precargar archivos y datos del cliente, da click en finalizar perfil del cliente, se sube a nuestra base de datos y una IA local en el servidor (o API a nube) le escupe el score del cliente, se le dan las recomendaciones de las cantidades (en valor de producto) de hasta cuanto debería ser consciente de aprobarle o de rechazarlo, ya con el perfil del cliente activo, tener un panel que sirva de plan de pagos, de esa manera es ahí en donde el encargado de antención al cliente en el área de créditos de cada microempresa empieza el prellenado del plan de pagos, supongamos, dentro del perfil del cliente le da al botón de nuevo plan de pagos, se abre la respectiva ventana emergente, y ahí los inputs son de qué producto quieren autorizar al cliente a pagos, costo total del producto, a cuantos meses se lo va a llevar con cuanto porcentaje de cat o interés, o si son meses sin intereses, es decir planes de pagos según su nivel de confianza, y entonces si tienen sentido los botones de aprobar crédito o no.

---
## 🔗 Enlaces Relacionados
*   [[]]
* 