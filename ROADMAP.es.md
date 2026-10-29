# Hoja de ruta de Mamba

Esta página muestra hacia dónde va Mamba: del editor VAG EDC15/EDC16 actual a un editor de reprogramación completo para cualquier binario de centralita.

Para pedir una función o informar de un error, abre una issue en [GitHub](https://github.com/bobnautic/Mamba/issues).

## En la 1.0

- La identidad de Mamba: nombre, logo de serpiente, tema verde y negro, gráficos del instalador y canal de actualización propio.
- El editor: detección automática de mapas en Bosch EDC15P, EDC15VM+, EDC16U1, EDC16U31 y EDC16U34; cualquier otro binario se abre con un proyecto WinOLS `.ols`, un `.xdf` de TunerPro o un mappack JSON; edición en tabla, 2D, 3D y en el hexdump; versiones y comparación; banco virtual; DTC on/off; corrección de checksum; exportación de mappack WinOLS.

## Previsto

- Deshacer y rehacer (Ctrl+Z / Ctrl+Y) en el editor.
- Un buscador de mapas para binarios sin definiciones: mapas candidatos propuestos a partir de la estructura del archivo (ejes, puntos crecientes, rejillas de 8 y 16 bits), nombrados y confirmados a mano.
- Exportar una lista de mapas como `.xdf` de TunerPro, e importación y exportación CSV de un mapa.
- Los mapas de dos versiones lado a lado en la ventana de comparación, con las diferencias resaltadas.
- Vista 2D al estilo WinOLS para matrices completas: una curva por fila.
- Checksums más allá de EDC15/EDC16, familia por familia a medida que se verifiquen.
- Mapas favoritos y una versión de referencia distinta de Ori.

## No previsto

- Eliminaciones relacionadas con emisiones (EGR off, DPF off, AdBlue/SCR off) y reprogramaciones de un clic. Mamba es un editor: ves los mapas y decides qué cambias.
- Cuentas o nube. Todo se queda en tu ordenador.
