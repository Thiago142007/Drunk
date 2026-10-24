# Avaliação do runtime Android do Drunk

**Data da investigação:** 2026-10-24. **Resultado:** **NO-GO provisório para iniciar o runtime de produto da opção B.** Nenhum backend foi selecionado e nenhuma aplicação Android foi iniciada nesta investigação. Isso registra falta de demonstração, não prova impossibilidade técnica.

A [especificação aprovada](superpowers/specs/2026-10-24-drunk-design.md), seções 5–7, exige código próprio Rust e uma composição Android/Linux demonstrada antes do produto. A decisão está na [ADR 0001](adr/0001-android-runtime-backend.md). A investigação foi limitada a documentação pública e inspeção local de leitura: sem instalar pacotes, baixar APKs, iniciar software Android ou alterar serviços/configuração do sistema.

## 1. Como interpretar as evidências

- **Documentado:** afirmação publicada pelo mantenedor, com link; não equivale a medição local.
- **Observado:** resultado dos comandos de leitura da seção 3, nesta máquina e nesta data.
- **Inferência:** avaliação de engenharia do Drunk, explicitamente indicada.
- **Não demonstrado:** falta evidência necessária; não deve ser tratado como aprovado.

Os links foram consultados em 2026-10-24. Páginas e branches móveis podem mudar; numa execução futura devem ser registrados commits, versões, hashes e uma cópia dos resultados utilizados. As fontes selecionadas são dos próprios projetos. Nenhuma alegação sobre componentes internos do Sober vem de terceiros ou da inspeção de seus binários.

## 2. Limite da evidência sobre o Sober

| Afirmação | Evidência e limite |
| --- | --- |
| O repositório público serve como issue tracker | **Documentado:** [README oficial](https://github.com/vinegarhq/sober). Não oferece a implementação do runtime. |
| Sober é fechado e experimental | **Documentado:** [site oficial](https://sober.vinegarhq.org/). A existência do produto não oferece código ou uma interface de incorporação ao Drunk. |
| O runtime especializado executa a versão Android do Roblox e faz a ponte Android/Linux | **Documentado:** [FAQ oficial](https://vinegarhq.org/Sober/FAQ/index.html). É uma descrição funcional, não uma especificação de APIs ou dependências internas. |
| É possível reproduzir esse resultado combinando bibliotecas públicas específicas | **Não demonstrado.** Não foi identificado, nas fontes acima, um contrato público para tal composição. |
| Sober usa ART, um linker, uma tradução de ABI ou uma ponte gráfica específica | **Desconhecido.** Não atribuímos essas escolhas ao Sober. |

**Inferência:** Sober demonstra que seus autores oferecem esse resultado, mas não demonstra que existe um backend reutilizável, licenciável e mantido que satisfaça B. Sua instalação local, registrada abaixo, também não valida um runtime próprio do Drunk.

## 3. Inspeção local reproduzível

Estes comandos somente consultam identificação do sistema, disponibilidade de programas e arquivos. Foram executados; comandos de lançamento da seção 5 não foram.

```bash
cat /etc/os-release
uname -srmo
lscpu
lspci -nnk
printf 'Session: %s\n' "${XDG_SESSION_TYPE:-unset}"
dpkg-query -W libgl1-mesa-dri mesa-vulkan-drivers libegl-mesa0
for tool in adb emulator waydroid crosvm qemu-system-x86_64 flatpak vulkaninfo glxinfo lspci; do
    command -v "$tool" || true
done
flatpak list --app --columns=application,version
ls -l /dev/kvm /dev/dri /dev/binderfs /dev/binder
for p in "$HOME/Android/Sdk/emulator/emulator" \
    /opt/android-sdk/emulator/emulator /usr/lib/android-sdk/emulator/emulator \
    /var/lib/waydroid/images "$HOME/.android/avd"; do
    if [ -e "$p" ]; then ls -ld "$p"; else printf 'Absent: %s\n' "$p"; fi
done
```

| Campo | Resultado observado |
| --- | --- |
| Distribuição/kernel/arquitetura | Ubuntu 26.04.1 LTS; `Linux 7.0.0-38-generic x86_64 GNU/Linux` |
| CPU | Intel Core i5-2400 @ 3.10 GHz; 4 CPUs lógicas |
| Flags relevantes presentes | `lm`, `nx`, `sse4_1`, `sse4_2`, `popcnt`, `cx16`, `lahf_lm`, `vmx`, `ept`, `avx`; `lscpu` informa VT-x |
| GPU/driver de kernel | AMD/ATI Park, Mobility Radeon HD 5430/5450/5470, PCI `1002:68e0`; subsistema XFX Radeon HD 5450; driver em uso `radeon` |
| Bibliotecas gráficas instaladas | `libgl1-mesa-dri`, `mesa-vulkan-drivers`, `libegl-mesa0`: `26.0.8-1ubuntu0.3` |
| Sessão/dispositivos | `XDG_SESSION_TYPE=wayland`; `/dev/dri/card0`, `/dev/dri/renderD128` e `/dev/kvm` existem |
| Binder | `/dev/binderfs` e `/dev/binder` não encontrados nesses caminhos; isso não determina suporte compilado ou possibilidade de carregar módulos |
| Ferramentas no PATH | `/usr/bin/adb`, `/usr/bin/flatpak`, `/usr/bin/lspci`; não encontrados `emulator`, `waydroid`, `crosvm`, `qemu-system-x86_64`, `vulkaninfo`, `glxinfo` |
| Caminhos adicionais | Nenhum dos cinco caminhos do último loop existe |
| Sober instalado | O inventário Flatpak lista `org.vinegarhq.Sober` versão `1.8.0`; não foi iniciado nem inspecionado internamente |
| Runtime/imagem/API de um candidato Drunk | **Não selecionados/não medidos** |
| Backend gráfico, input, áudio, lançamento, saída, logs Android | **Não executados/não medidos** |

**Limites:** a busca não cobriu todos os diretórios do disco nem dispositivos remotos. A ausência no PATH e nos caminhos usuais não prova ausência global. A existência de `/dev/kvm` não prova acesso utilizável; pacotes Mesa instalados não provam suporte Vulkan/GLES desta GPU. Não foi executado `adb devices`: o ADB pode iniciar um servidor automaticamente ([documentação ADB](https://developer.android.com/tools/adb)). Não há log de execução Android nem captura de janela porque nenhuma execução foi autorizada nesta etapa.

**Conclusão local:** não foi identificado um candidato B pronto para demonstrar o lançamento com as ferramentas verificadas. ADB sozinho não é runtime. Mesmo uma execução bem-sucedida do Sober não serviria como prova do adaptador próprio.

## 4. Modelos candidatos

### 4.1 Adaptador Linux próprio consumindo componentes AOSP

Este é um **modelo de pesquisa**, não o nome de um backend pronto. A [arquitetura AOSP](https://source.android.com/docs/core/architecture) documenta framework, serviços e componentes do sistema Android. A documentação de [ABIs do NDK](https://developer.android.com/ndk/guides/abis) inclui `x86_64`. Isso confirma componentes e uma ABI, não um SDK público para executar APKs arbitrários em um processo Linux comum.

| Critério | Avaliação |
| --- | --- |
| Responsabilidade | **Proposta B:** Drunk mantém supervisor/adaptador Rust; componentes Android ficam externos. Ainda falta uma lista fechada de componentes, versões, contratos e responsáveis por correções. |
| x86-64 | ABI Android documentada; compatibilidade da composição proposta **não demonstrada**. Uma biblioteca x86-64 não fornece automaticamente o ambiente Android esperado pelo aplicativo. Esta última conclusão é **inferência**. |
| Gráficos/janela | AOSP descreve sua [arquitetura gráfica](https://source.android.com/docs/core/graphics/architecture); não foi demonstrada uma ponte dessa composição para uma janela Wayland/X11. Backend GLES/Vulkan e ciclo de vida da superfície continuam indefinidos. |
| Input/áudio | Há documentação do [pipeline de input Android](https://source.android.com/docs/core/interaction/input) e de [áudio, serviços, Binder e HAL](https://source.android.com/docs/core/audio). Tradução de eventos desktop e integração ao servidor de áudio Linux para B **não demonstradas**. |
| Privilégios/kernel | Dependem dos componentes e do contrato de IPC escolhidos. **Não determinados**; não assumir funcionamento sem Binder, permissões de dispositivo ou serviços auxiliares. |
| Manutenção/releases | AOSP publica [tags e builds](https://source.android.com/docs/setup/reference/build-numbers), incluindo Android 17/API 37. **Inferência:** manutenção do AOSP não constitui manutenção de uma composição Linux própria; esta não tem release nem suporte demonstrado. |
| Licença/distribuição | AOSP usa predominantemente Apache 2.0, com exceções, incluindo kernel GPLv2 ([licenciamento AOSP](https://source.android.com/docs/setup/contribute/licenses)). É necessário inventário por componente/artefato, avisos e obrigações aplicáveis. Nenhuma licença global da composição foi aprovada. |
| Encaixe em B | **Conceitualmente sim; tecnicamente não demonstrado.** Não autoriza reimplementar grandes partes do Android nem escrever componentes próprios em C/C++. |

**Inferência decisiva:** há documentação das partes, mas falta uma integração mínima delimitada que una instalação, execução, janela, input, áudio e encerramento. Não há base para escolher bindings ou iniciar o runtime de produto agora.

### 4.2 Waydroid como referência ou backend externo

**Documentado:** Waydroid executa um **Android completo em container Linux**, usando namespaces e uma imagem customizada baseada em LineageOS; sua página informa Android 13 ([visão geral oficial](https://docs.waydro.id/)). A [tabela AOSP](https://source.android.com/docs/setup/reference/build-numbers) associa Android 13 à API 33. Isso descreve o projeto, não uma imagem instalada ou verificada localmente.

| Critério | Avaliação |
| --- | --- |
| Responsabilidade | Waydroid administra container, imagem e integração Android. Seu [CLI público](https://docs.waydro.id/usage/install-and-run-android-applications) instala e inicia apps; um Drunk que apenas o chamasse seria uma integração com outro ambiente. |
| x86-64 | A documentação de apps alerta que APKs somente ARM não funcionam em x86-64. Usar app com ABI compatível; nenhuma tradução ARM foi selecionada. |
| Gráficos/janela | [Instalação oficial](https://docs.waydro.id/usage/install-on-desktops) requer sessão Wayland no fluxo Ubuntu/Debian; [propriedades públicas](https://docs.waydro.id/usage/waydroid-prop-options) incluem integração de múltiplas janelas. O caminho gráfico efetivo precisa ser medido. |
| Input/áudio | O [guia de diagnóstico](https://docs.waydro.id/debugging/troubleshooting) reconhece problemas de teclado físico. O [manifesto Debian upstream](https://github.com/waydroid/waydroid/blob/main/debian/control) depende de `pipewire-pulse` ou `pulseaudio`. **Inferência:** são interfaces a avaliar como parte do backend inteiro; não demonstram uma biblioteca estável embutível para Rust. |
| Privilégios/kernel | Dependência de LXC no manifesto; instalação e gestão do serviço/container incluem operações privilegiadas. O diagnóstico documenta falhas de Binder. Configuração exata do kernel e permissões mínimas ainda precisam ser verificadas para uma versão fixada. Não se assume instalação em Flatpak sem integração ao host. |
| Manutenção/releases | [Releases upstream](https://github.com/waydroid/waydroid/releases) mostra `1.6.3` como latest nesta consulta, com correções de encerramento/reboot e desativação de Vulkan no driver legado `radeon`. **Inferência:** há evidência de manutenção, mas sem garantia de suporte para nossa GPU ou prazo futuro. |
| Licença/distribuição | [LICENSE do projeto](https://github.com/waydroid/waydroid/blob/main/LICENSE) contém GPLv3. Imagens Android e dependências exigem inventário próprio; não herdam automaticamente uma licença única. Não foi aprovada redistribuição de imagem com serviços Google, Roblox ou outros componentes proprietários. |
| Encaixe em B | **Não, se usado como está por meio de seu CLI.** Isso corresponde à opção A da especificação. Extrair componentes e manter uma integração própria seria uma nova pesquisa B, sem prova nesta investigação. |

Waydroid permanece uma referência técnica útil. Selecioná-lo como backend completo exigiria uma nova decisão de arquitetura do usuário, além da validação técnica; não foi selecionado aqui.

### 4.3 Sistema Android completo em VM: Android Emulator

Representante documentado do modelo de VM completa: [Android Emulator](https://developer.android.com/studio/run/emulator), com dispositivo virtual/imagem Android. O modelo de container completo já é representado por Waydroid; não se presume equivalência entre container e VM.

| Critério | Avaliação |
| --- | --- |
| Responsabilidade | O emulador/imagem externos fornecem o dispositivo Android completo. Drunk teria responsabilidade de orquestração, não de adaptar diretamente APIs Android ao Linux. |
| x86-64 | A [documentação de aceleração](https://developer.android.com/studio/run/emulator-acceleration) descreve imagens x86/x86-64 e aceleração de VM no host compatível. A imagem concreta ainda precisa ser escolhida e fixada. |
| Gráficos/janela | O emulador oferece janela própria e renderização por hardware ou software, configurável com `-gpu`; GLES/Vulkan dependem de imagem, driver e modo. Não há medição nesta máquina. |
| Input/áudio | O frontend do dispositivo virtual fornece controles de interação; a [referência de linha de comando](https://developer.android.com/studio/run/emulator-commandline) documenta opções de áudio. O mapeamento desktop e a saída audível precisam ser demonstrados; não foi definida ponte própria Rust. |
| Privilégios/kernel | Linux usa KVM para aceleração, com virtualização de CPU e permissões apropriadas. A disponibilidade foi apenas parcialmente observada; nenhum dispositivo foi aberto nem módulo/configuração alterado. |
| Manutenção/releases | [Notas oficiais](https://developer.android.com/studio/releases/emulator) registram `37.2.12 Stable (Oct 1, 2026)`. Há manutenção publicada; isso não resolve licença, integração ou compatibilidade de um produto Drunk. |
| Licença/distribuição | Os [termos do SDK](https://developer.android.com/studio/terms), §§3.1–3.5, limitam o uso concedido e a redistribuição, preservando licenças próprias de componentes abertos. Não presumir que binários/imagens do SDK podem ser incorporados ao produto. Um build de fontes abertas exigiria inventário separado. A adequação da distribuição continua **não demonstrada**. |
| Encaixe em B | **Inferência:** um lançador do emulador completo não demonstra o adaptador especializado B. Pode servir como controle de laboratório, após resolver condições de uso; adoção como produto exige revisão da arquitetura. |

## 5. Protocolo manual futuro — não executado

### Aplicativos e preparação

Aplicativo inicial proposto: **Native Audio**, do repositório oficial [android/ndk-samples](https://github.com/android/ndk-samples/tree/main/native-audio). Ele reproduz/grava áudio por JNI/OpenSL ES; seu [README](https://github.com/android/ndk-samples/blob/main/native-audio/README.md) informa que OpenSL ES está depreciado e que o player URI tem problema conhecido. Usar apenas reprodução local; não usar microfone nem streaming neste protocolo. A [licença Apache 2.0](https://github.com/android/ndk-samples/blob/main/LICENSE) permite uso/reprodução sob suas condições. Preservar licença, avisos e licenças das dependências da revisão escolhida.

Complemento gráfico: **Hello GL2**, do mesmo repositório/licença, cujo [README](https://github.com/android/ndk-samples/blob/main/hello-gl2/README.md) descreve um triângulo GLES 2.0 com atividade Java e JNI. Ele verifica um caminho gráfico básico, não GLES 3, Vulkan ou Roblox. Os aplicativos são artefatos de terceiros usados como sondas externas, não código C/C++ mantido pelo Drunk.

Antes de executar, o responsável deve preparar um ambiente permitido, fixar commit completo do repositório e versões do JDK/Gradle/SDK/NDK, registrar comandos de build e produzir APKs locais com ABI `x86_64`. Registrar SHA-256 dos APKs, identidade de assinatura, pacote/atividade e min/target API extraídos dos artefatos. Não inferir estes valores a partir do nome do diretório. O uso dos componentes de build deve respeitar seus termos. Esta investigação não baixou nem compilou os samples.

### Registro obrigatório por execução

| Grupo | Campos a preencher; estado nesta investigação |
| --- | --- |
| Host | Data, distro, kernel, arquitetura, CPU/flags, RAM, GPU/PCI, driver de kernel e userspace, sessão/compositor; identificação parcial na seção 3 |
| Runtime | Nome, versão/commit, origem, hash de cada imagem, API level, fingerprint, patch de segurança, ABI; **não selecionado** |
| Licenças | Inventário dos componentes, origem, permissão de uso/distribuição, avisos e fonte correspondente quando exigida; **inventário não concluído** |
| Aplicativos | Commit upstream, versões/comandos de build, pacote/atividade, ABI, APIs, hash e assinatura; **não preparados** |
| Gráficos | Backend solicitado e efetivo, renderer/vendor/API, aceleração ou software, tamanho da janela; **não medido** |
| Interação | Mouse/toque, teclado, foco e comportamento observado, dispositivos e servidor de áudio, saída audível; **não medidos** |
| Ciclo de vida | Comando exato, tempos de boot/primeiro frame, resultado de lançamento, fechar/relançar, stop, códigos de saída, processos restantes; **não executado** |
| Evidências | Captura de janela, logs Android/runtime/host com horários e segredos removidos; **não produzidas** |

### Sequência e critério de aprovação

1. Repetir a inspeção de host e preencher todos os campos. Ausência de medição deve permanecer explícita. Fixar a versão do candidato e documentar instalação, inicialização e parada a partir de sua documentação oficial. Um candidato B deve ter uma composição e interface concretas; a ausência disso é falha do gate antes do lançamento.
2. Iniciar apenas a instância de laboratório prevista e registrar o comando e sua saída. Em uma VM de controle, a referência documenta `emulator -avd NOME -gpu MODO`; registrar `NOME`/`MODO` reais e versão, pois a documentação também anuncia migração para Android CLI. Uma execução de controle não valida B.
3. Onde o candidato documentar ADB, selecionar explicitamente seu serial. Exemplo de operações futuras: `adb -s SERIAL shell getprop ro.build.version.sdk`, `adb -s SERIAL shell getprop ro.build.fingerprint`, `adb -s SERIAL shell getprop ro.product.cpu.abilist`, `adb -s SERIAL install CAMINHO_APK` e `adb -s SERIAL shell am start -W -n PACOTE/ATIVIDADE`. Substituir cada marcador por valor registrado. Referência: [ADB oficial](https://developer.android.com/tools/adb). Não presumir suporte ADB em um adaptador B; se faltar, registrar a interface equivalente e seus logs.
4. Abrir Native Audio, verificar janela e usar o mouse para acionar reprodução local. Confirmar áudio audível; registrar o efeito de foco e teclado. Marcar funções não exercitadas ou não suportadas como tal. Executar Hello GL2 e verificar o triângulo e o renderer efetivo; não confundir renderização por software com aceleração da GPU.
5. Manter cada app aberto por 60 segundos, alternar foco, fechar e relançar duas vezes. Registrar falhas, congelamentos e ações necessárias. Depois parar a sessão/runtime pela interface documentada; registrar códigos e processos antes/depois. Processo Android em cache após fechar uma atividade deve ser distinguido de processo do runtime abandonado após parar a sessão.
6. Capturar logs com horários; se ADB estiver disponível, usar `adb -s SERIAL logcat -d` após a execução. Registrar também saída do supervisor/backend e qualquer crash. Não publicar tokens, cookies, contas ou áudio pessoal. Guardar evidência local suficiente para um terceiro repetir a execução.
7. Aprovar somente quando houver janela, lançamento real, resultado de input/áudio/gráficos, parada controlada e repetibilidade, **além** de composição B documentada e revisão de licenças/manutenção. Falha, campo crítico desconhecido ou ausência de lançamento mantém **NO-GO**. Sucesso dos samples não prova compatibilidade com Roblox.

O protocolo de Waydroid seria `waydroid app install CAMINHO_APK` e `waydroid app launch PACOTE`, segundo o CLI citado. Só constitui avaliação de referência/opção A; não é uma alternativa silenciosa ao gate B.

### Validação posterior separada do Roblox

Somente após o gate técnico, definir e documentar uma origem oficial permitida do aplicativo, identidade do editor/assinatura, versão, ABI e integridade do bundle. O ponto de entrada oficial é o [site de download Roblox](https://www.roblox.com/download). **Nenhum método de obtenção automatizada ou redistribuição do APK foi aprovado aqui.** Se não houver método permitido e verificável para o ambiente escolhido, interromper esta fase.

Usar o aplicativo oficial **sem modificações**, em uma execução separada com a mesma ficha de ambiente. O usuário conduz autenticação; registrar apenas resultados necessários de inicialização, login, entrada em experiência, gráficos/input/áudio e saída, removendo segredos dos logs. Rejeição por integridade, anti-cheat, attestation ou bloqueio de plataforma deve ser registrada como incompatibilidade. Não alterar APK, simular integridade, contornar verificações nem prometer acesso a experiências. Estas são restrições do projeto, não uma alegação de compatibilidade ou permissão concedida pelo Roblox.

## 6. Resultado do gate e condições para reabertura

| Requisito | Resultado |
| --- | --- |
| Evidência pública do Sober delimitada | Concluído, seção 2 |
| Comparação de modelos com fontes primárias | Concluído, seção 4; lacunas explícitas |
| Backend B concreto e sustentável | **Não demonstrado** |
| Licenças de uma composição selecionada | **Não demonstradas**; não há composição selecionada |
| Janela e app Android de teste executados | **Não executados** |
| Protocolo manual reproduzível | Definido, seção 5; exige preparação futura |
| Decisão | **NO-GO provisório**; pausar implementação do runtime/produto |

Para reabrir B: apresentar lista de componentes/versionamento e interfaces públicas, distribuição compatível com suas licenças, responsáveis pela manutenção e evidências completas do protocolo em uma máquina Linux x86-64. Se o caminho demonstrado for somente Waydroid ou outro Android completo pronto, solicitar uma nova decisão do usuário sobre a opção A antes de implementar o produto. Não há justificativa para começar runtime, ponte gráfica ou instalador enquanto esse gate permanecer aberto.
