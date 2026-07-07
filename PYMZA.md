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

## 🧠 Brainstorming y Notas Sueltas
*   Las empresas crean un perfil financiero ligado a la ine de los solicitantes, en caso de ser nuevo perfil y basado en históricos brindar a la empresa niveles de confianza del solicitante de crédito. En caso de ya haberse dado de alta con otra empresa por otro producto a crédito analizar su perfil y su nivel de confianza, si resulta ser cliente moroso y además se encuentra en calidad de desaparecido la empresa activa una pequeña alerta o flag en el software que anuncia que fue visto en ese local. 
* El Motor de Datos Alternativos: El cliente típico de muchas PYMES a veces opera en la informalidad o no tiene tarjetas de crédito bancarias. En lugar de solo consultar un historial crediticio clásico, el backend podría perfilar el riesgo analizando otras fuentes. Por ejemplo, procesar comprobantes de ingresos o leer el historial de pagos de servicios básicos (luz, agua, telecomunicaciones). Si un cliente paga sus servicios impecablemente, el algoritmo le asigna un "Score de Confianza" alto, creando un modelo de riesgo alterno válido para el hackaton.
* La Red de Alerta Temprana (El "Sistema Inmunitario" de PYMES): Sistema de reputación colaborativo. Si un "Sujeto A" comete fraude o desaparece con una deuda en una PYME, ese nodo actualiza la base de datos. Cuando el sujeto intente sacar crédito en otro negocio de la red, el sistema arroja la alerta temprana. Esto crea una red de protección comunitaria que las herramientas corporativas no suelen tener. (blockchain? me sonó a un buen momento para analizar el uso de estas herramientas sobre la red de ethereum o algo así)
* ​Onboarding con Ciberseguridad: Integrar un microservicio rápido de validación por OCR para verificar que la identificación sea legítima y evitar la suplantación de identidad. Esto ataca directamente el rubro de "Seguridad informática y prevención de fraude".

---
## 🔗 Enlaces Relacionados
*   [[]]
* 