# Roteiro do Mamba

Esta página mostra para onde o Mamba vai: do atual editor VAG EDC15/EDC16 a um editor de remapeamento completo para qualquer binário de centralina.

Para pedir uma função ou relatar um erro, abra uma issue no [GitHub](https://github.com/bobnautic/Mamba/issues).

## Na 1.0

- A identidade do Mamba: nome, logo de serpente, tema verde e preto, imagens do instalador e canal de atualização próprio.
- O editor: deteção automática de mapas em Bosch EDC15P, EDC15VM+, EDC16U1, EDC16U31 e EDC16U34; qualquer outro binário abre com um projeto WinOLS `.ols`, um `.xdf` do TunerPro ou um mappack JSON; edição em tabela, 2D, 3D e no hexdump; versões e comparação; dinamómetro virtual; correção de checksum; exportação de mappack WinOLS.

## Planeado

- Desfazer e refazer (Ctrl+Z / Ctrl+Y) no editor.
- Um localizador de mapas para binários sem definições: mapas candidatos sugeridos a partir da estrutura do ficheiro (eixos, pontos crescentes, grelhas de 8 e 16 bits), nomeados e confirmados à mão.
- Exportar uma lista de mapas como `.xdf` do TunerPro, e importação e exportação CSV de um mapa.
- Os mapas de duas versões lado a lado na janela de comparação, com as diferenças destacadas.
- Vista 2D ao estilo WinOLS para matrizes completas: uma curva por linha.
- Checksums além de EDC15/EDC16, família a família à medida que forem verificados.
- Mapas favoritos e uma versão de referência diferente de Ori.

## Não planeado

- Remoções ligadas às emissões (EGR off, DPF off, AdBlue/SCR off) e remapeamentos de um clique. O Mamba é um editor: vê os mapas e decide o que muda.
- Contas ou nuvem. Tudo fica no seu computador.
