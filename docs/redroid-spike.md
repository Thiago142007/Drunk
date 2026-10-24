# Spike redroid controlado pelo Drunk

Data: 2026-10-24. Este protótipo adiciona um controlador Rust pequeno para um
Android externo. O código próprio do Drunk usa Rust, `minifb` e `image` para
exibir screenshots e encaminhar eventos básicos; Docker, ADB e a imagem Android
são componentes externos e não são inteiramente Rust.
O **NO-GO de produto permanece**: esta execução não seleciona o backend final,
resolve o inventário de licenças nem demonstra compatibilidade com Roblox.

**Atualização do controlador (2026-10-24):** o digest padrão agora é o redroid
Android 13 `sha256:36d6d21bcf7e92d78eabaa6f1748e5cf0e9fb15176c0091794168be012c5c22e`
(API 33), necessário para a captura `playback` de áudio do scrcpy. O relato
Android 12 abaixo é histórico; o teste Android 13 e a correção de áudio estão
registrados no fim deste documento. Esta troca de imagem do protótipo não aprova
redroid como backend de produto.

## Ambiente observado

O laboratório existente usa Linux x86-64 com `binder_linux` carregado e Docker.
O container `drunk-redroid` iniciou Android 12 e ficou acessível por ADB em
`127.0.0.1:5555`. A imagem foi fixada em:

```text
redroid/redroid@sha256:a6c464bbedcf1dcb67dbf91f329fbb19bee5b50631f0ca6bda6ed7c41b0e64e2
```

O comando que permitiu o boot usa `--platform linux/amd64`, `--privileged`,
`--restart=no`, o volume nomeado `drunk-redroid-data:/data` e somente a publicação
`127.0.0.1:5555:5555`. Não há argumentos adicionais depois da imagem.
Mapear manualmente dispositivos Binder do host impediu o boot nesta investigação;
o comando funcional omite `--device` e deixa a imagem criar seu próprio BinderFS
interno. O host também tinha uma montagem BinderFS temporária preparada pelo
usuário; sua necessidade não foi demonstrada por essa execução.

Esses resultados descrevem o container preparado manualmente no laboratório. O
controlador Rust foi então exercitado: `status` identificou o container, `start`
confirmou a reutilização idempotente após validar sua configuração, `demo` abriu
Settings, e `stop` o encerrou. O controlador não foi usado para criar o container
inicial. O build e as consultas de ajuda/estado são verificações distintas do
ciclo completo.

## Uso da CLI

Requisitos: Docker acessível ao usuário, ADB no PATH e `binder_linux` previamente
carregado pelo administrador. A CLI não chama `sudo`, carrega módulos nem monta
sistemas de arquivos. Ela só opera no contexto Docker `default` com endpoint
local Unix; variáveis `DOCKER_HOST` e `DOCKER_CONTEXT` são recusadas para não
criar ou parar um container em outro host.

```bash
cargo build
target/debug/drunk redroid status
target/debug/drunk redroid start --allow-privileged
target/debug/drunk redroid demo
target/debug/drunk redroid view
target/debug/drunk redroid stream-probe --seconds 5
target/debug/drunk redroid stream --seconds 30
target/debug/drunk redroid audio
target/debug/drunk redroid stop
```

- `start` exige `--allow-privileged`, verifica `/proc/modules` e `docker info` e
  cria o container com os parâmetros fixos acima. Um container já em execução
  com a mesma imagem, modo privilegiado bridge, ADB somente em loopback, política sem
  reinicialização e volume nomeado retorna sucesso sem reiniciá-lo. Para um
  container gerenciado em `exited`, `dead` ou `created`, valida a configuração,
  remove somente o container parado e o recria usando o mesmo volume e imagem.
  Estados inesperados e containers com configuração divergente exigem inspeção
  manual. `demo` também exige essa configuração. O sucesso de `start` informa que
  Docker iniciou o container; não garante que Android já terminou o boot.
- `status` informa o estado Docker ou `absent`. Falha de acesso ao Docker é erro,
  não ausência de container.
- `demo` conecta ADB ao endpoint fixo e solicita a atividade de Configurações
  Android por `am start -a android.settings.SETTINGS`. Pode falhar enquanto Android
  estiver inicializando; execute novamente após o boot. Antes de conectar, valida
  que o container corresponde à configuração gerenciada e rejeita configuração
  divergente. A CLI não aceita APKs,
  não instala aplicativos e não executa Roblox.
- `view` cria uma janela Linux com `minifb`, captura o framebuffer Android como
  PNG por ADB a cada 250 ms (até 4 FPS) e redimensiona a imagem para caber em
  540×680. Um clique esquerdo envia `input tap`; teclas básicas (letras, números,
  setas, Enter, espaço, Tab e Backspace) são enviadas como keyevents. Escape
  fecha a janela. O input é encaminhado como eventos discretos; manter uma
  tecla pressionada, arrastar/gestos, clipboard e áudio não estão implementados.
  A taxa de captura é baixa e cada frame inicia um processo ADB, portanto não é
  um caminho de streaming para jogos.
- `stream-probe` pede ao Android para capturar H.264 bruto por até 30 segundos
  via `adb shell -T`, conta bytes e exige NALs Annex-B de SPS, PPS e quadro.
  Separa stderr do stream e reporta diagnóstico remoto quando `screenrecord`
  falha. Informa throughput aproximado e tempo até o primeiro byte; não
  decodifica nem salva vídeo. Foi adicionado para confirmar a disponibilidade
  do transporte antes de integrar um decoder.
- `stream` abre uma prévia contínua via `adb shell -T screenrecord`, decodifica
  cada NAL no crate Rust `hibernia` 0.1.0 e converte os planos YUV 4:2:0 para a
  janela `minifb`, convertendo direto para a resolução da janela em vez de
  materializar um frame RGB em tamanho integral. Um decoder auxiliar libera o
  primeiro IDR para que uma tela estática também apareça sem esperar outro quadro;
  o decoder contínuo preserva suas referências. O limite selecionável é 1–180
  segundos (30 por padrão), pois `screenrecord` tem duração máxima finita.
  Escape fecha a janela e o processo local ADB é encerrado e recolhido. O decoder
  fica em uma thread separada; um panic do Hibernia vira erro compreensível. A
  fila de frames tem capacidade um e substitui frames antigos para não acumular
  atraso se a janela não acompanhar o decoder. Esta é uma experiência de
  visualização e não implementa os controles completos de um jogo.
- `audio` captura reprodução de apps Android via scrcpy `playback` (Android 13+), sem abrir
  uma segunda janela. Requer `scrcpy` instalado e disponível no `PATH`; ele é um
  componente externo e sua licença/versão deve ser incluída em qualquer revisão
  de distribuição. O comando mantém um sink de vídeo oculto em `/dev/null` para
  o scrcpy continuar ativo enquanto encaminha áudio. Ctrl+C encerra o helper.
  O caminho `REMOTE_SUBMIX` do Android 12 foi substituído após produzir silêncio
  no monitor HDMI. No fim deste documento, Android 13 gerou sinal e o usuário
  confirmou ter ouvido o HELLO no caminho integrado.
- `stop` valida a configuração, para `drunk-redroid` e consulta o código de saída
  do container. Depois de parar, `start` pode recriar o container gerenciado e
  reutilizar o volume Android existente. Se Docker precisou enviar SIGKILL após o timeout, a CLI informa
  isso explicitamente; no redroid medido aqui, `Exited (137)` ocorreu mesmo
  com timeout de 60 segundos. O container inativo é removido após validar sua
  configuração; volume e imagem permanecem. A CLI não descarrega Binder nem
  desmonta uma montagem temporária do host.

O modo privilegiado reduz o isolamento do container. O usuário autorizou esse uso
temporário para o laboratório; a opção deve ser fornecida em cada chamada de
`start`. A publicação ADB fica restrita ao loopback do host, mas processos locais
com acesso a esse endpoint ainda podem controlar o Android.

## Stream H.264 em Rust: estado experimental

O caminho de screenshots valida a janela, mas não serve para jogos. A prévia
H.264 experimental usa uma leitura contínua de
`adb shell -T screenrecord --output-format=h264 -`. A implementação AOSP do
`screenrecord` confirma que o formato H.264 bruto aceita `-` como saída em
stdout ([código AOSP](https://android.googlesource.com/platform/frameworks/av/+/297eaacfa6/cmds/screenrecord/screenrecord.cpp)).
O probe confirmou a entrega desse formato pela imagem redroid Android 12 nesta
execução; a sessão de exibição com decoder continua sem validação de runtime.

O comando `stream` integra o crate publicado `hibernia = 0.1.0` (MIT) em uma
thread de decoder. Essa publicação está em desenvolvimento inicial (WIP),
declara suporte somente ao perfil Baseline e contém caminhos ainda não
implementados (`todo!()`), que podem provocar panic. A thread captura esses
panics e os converte em encerramento com diagnóstico, mas isso não garante que
todo bitstream seja suportado nem que o decoder seja estável. O código está
fixado pela versão do crates.io e `Cargo.lock`; não depende de uma API git não
publicada.

O decoder recebe unidades Annex-B separadas, converte os planos YUV 4:2:0 usando
strides e origens do Hibernia e produz RGB para `minifb`. Uma fila limitada a um
frame descarta o antigo quando o decoder avança mais rápido que a janela. O
processo local ADB é encerrado e recolhido ao fechar a janela ou ao encontrar um
erro; `screenrecord` também para ao alcançar o tempo limite selecionado (30 s por
padrão, até 180 s). Não há reconexão automática para streams longos.

O probe H.264 executado contra o redroid Android 12 recebeu 1.031.795 bytes em
5,27 s (0,19 MiB/s), primeiro byte em 386 ms, e encontrou NALs Annex-B SPS, PPS e
de imagem. Em seguida, `redroid stream --seconds 5` decodificou ao menos um frame,
abriu a janela 720×1280 e encerrou com código 0 ao atingir o limite do stream.
O conteúdo da janela não foi inspecionado visualmente de forma independente.
FPS, fluidez, latência ponta a ponta, uso de CPU e estabilidade por vários
minutos não foram medidos. O uso de decoder inteiramente Rust não altera os
gates de runtime, licenças e plataforma.

### Perfil de CPU do viewer

Uma medição de 30 s com `perf stat` no caminho `redroid view` identificou a
redimensionação Triangle da crate `image` como custo dominante. No build debug,
foram 19,27 s de `task-clock` em 30,14 s de relógio (64,0% de um núcleo lógico).
Após trocar essa etapa por amostragem nearest-neighbor e evitar a conversão
intermediária quando o frame já é RGBA, uma repetição debug marcou 6,85 s em
30,14 s (22,7% de um núcleo, queda de cerca de 64%). O build release marcou
1,09 s em 30,14 s (3,6% de um núcleo). São execuções únicas no mesmo host, sem
medição de FPS ou latência; nearest-neighbor pode parecer menos suave ao reduzir
a imagem.

O perfil H.264 é outro caminho: `redroid stream --seconds 30` em release marcou
13,44 s de `task-clock` em 30,16 s (44,6% de um núcleo); uma amostra debug
anterior marcou cerca de 102%. A conversão YUV→RGB e o decoder Hibernia aparecem
entre os custos relevantes. Uma execução por configuração não basta para
caracterizar desempenho ou fluidez do stream.

Depois da conversão direta para o tamanho da janela e da redução do stream a
macroblocos H.264 de 16 pixels, uma execução release com Hello GL2 marcou
5,15 s de `task-clock` em 30,62 s (16,8% de um núcleo), contra 11,40 s
(37,5%) na execução anterior em resolução integral. São execuções únicas, sem
mediana e sem FPS/latência; a diferença é evidência preliminar, não uma garantia.

O viewer H.264 usa agora a dimensão reduzida e arredondada a macroblocos: 384×672 em retrato e 528×304 em paisagem. Em 384×680, o decoder Hibernia ignorava o crop do SPS e criava 8 linhas extras; a resolução ajustada evita essa área de padding. Cliques no viewer H.264 enviam toques ADB escalados à tela Android, e algumas teclas têm mapeamento ADB de evento único. Toques e keyevents básicos não equivalem a controles contínuos ou mapeamento de gamepad.

Uma tela estática do Native Audio gerou apenas SPS, PPS e um NAL IDR. O Hibernia
mantém esse primeiro quadro em seu DPB de referência, portanto o viewer anterior
esperava até 20 s por um quadro que não chegava. O Drunk agora decodifica esse
IDR em uma instância auxiliar, a libera e mantém o decoder de streaming intacto.
A captura de resolução reduzida mostrou que Hibernia ignora o frame crop do SPS:
384×680 era exposto como 384×688, com linhas de padding no final. O tamanho
passou a ser alinhado a macroblocos: 384×672 em retrato e 528×304 em paisagem.
O teste do fixture estático decodificou um frame 384×672, e Hello GL2 abriu
uma janela 384×672 por 30 s. Cliques enviam toques ADB escalados para a resolução Android;
teclas mapeadas enviam keyevents individuais. O decoder de preview ainda tem
suporte WIP a Baseline apenas.

Em uma repetição posterior autorizada, `redroid stream --seconds 30` também abriu
a janela 720×1280 e permaneceu até o limite configurado. Uma amostra de processo
aos 28 s mostrou o Drunk em aproximadamente 92% de CPU e 1,3% de memória, com
`screenrecord` em 0,4% de CPU. É apenas uma amostra, sem média de CPU, FPS ou
latência; o conteúdo da janela não foi inspecionado independentemente. O estado
Docker após `drunk redroid stop` foi `Exited (137)`. `docker container stop --time
60` também deixou `Exited (137)` após 62,37 s. A imagem usa `/init` como entrypoint
e não define `StopSignal`; `adb shell reboot -p` retornou `Done`, mas o container
permaneceu em execução. A causa exata não foi confirmada; parada graciosa não foi
demonstrada. O CLI agora valida e recria o container parado preservando o volume.
Depois, um teste real confirmou exit 137, restart, novo boot Android e persistência
de um marcador em `/data/local/tmp`. Esse ciclo confirma recuperação após parada,
mas não torna o shutdown gracioso.

Também foram compilados e lançados os samples oficiais `Hello GL2` e `Native
Audio` do Android NDK Samples. GL2 desenhou o triângulo e informou ANGLE sobre
Vulkan SwiftShader; isso é renderização por software. O botão local HELLO do
Native Audio criou um track no AudioFlinger com rota Android `speaker`, sem
underruns na amostra. A permissão `RECORD_AUDIO` ficou negada. Scrcpy 3.3.4
falhou em `--audio-source=playback` com `NoSuchFieldException: MIX_ROLE_PLAYERS`,
como esperado para Android 12: essa fonte requer Android 13+. A fonte compatível
`--audio-source=output` funcionou quando mantida com vídeo oculto e gravação
descartada em `/dev/null`. O comando `drunk redroid audio` iniciou esse helper;
após toques HELLO, PipeWire mostrou um sink-input scrcpy ativo, a saída HDMI ficou
`RUNNING` e um PCM ALSA do host também ficou `RUNNING`. Isso demonstra entrega ao
stack de áudio do host, mas o usuário ainda não confirmou que ouviu o som. O
upstream redroid documenta componentes `r_submix`, embora WebRTC/áudio ainda seja
uma limitação conhecida ([issue 18 do projeto](https://github.com/remote-android/redroid-doc/issues/18)).
Cada app permaneceu aberto por 60 s e foi encerrado e relançado duas vezes sem
`FATAL EXCEPTION` nos logs consultados.

Não foi validado Roblox, som audível por uma pessoa, aceleração gráfica física ou
controle de jogo nesta experiência. Nenhuma compatibilidade é prometida. `view`
permanece como caminho separado de screenshots lentos para diagnóstico; `stream`
é a prévia H.264 experimental medida acima e ainda apresenta custo de CPU alto.

## Limpeza do laboratório

Após cada rodada, o container `drunk-redroid`, o volume `drunk-redroid-data` e a
imagem fixada foram removidos, e o endpoint `127.0.0.1:5555` foi desconectado.
Na limpeza inicial havia também um endpoint ADB `emulator-5554`, que foi deixado
intacto; na última limpeza `adb devices -l` ficou sem dispositivos. O servidor ADB
em si não foi encerrado.

Na verificação posterior, `binder_linux` ainda estava carregado, mas não havia
montagem ativa em `/dev/binderfs`. O spike não descarrega o módulo. Se o usuário
quiser removê-lo, deve primeiro confirmar que nenhum outro serviço local depende
dele e então executar no host:

```bash
sudo umount /dev/binderfs
sudo modprobe -r binder_linux
```

A montagem BinderFS não foi necessária para o container funcional demonstrado;
ela foi preparada durante a investigação. O módulo deve ser mantido se outro
serviço local depender dele.

## Limites deste marco

O viewer H.264 abriu uma janela por 30 s. Após reduzir o stream a 384×672, uma
execução marcou 16,8% de um núcleo lógico; FPS e latência continuam sem medida. Hello GL2 e Native Audio foram inspecionados por
capturas ADB. O encaminhamento de um toque físico pelo viewer `view` foi
confirmado no teste posterior descrito abaixo; teclado contínuo, controles de jogo,
alternância de foco e estabilidade prolongada não foram validados. O track de
áudio Android não comprova som audível no host. A renderização observada foi
SwiftShader software. O spike não inclui espera automática de boot, recuperação
de sessão em execução, atualização ou distribuição de imagens Android.

Na primeira tentativa de input, a janela de 900 px de altura não foi localizada
no host cuja tela tem 768 px de altura. O limite do viewer foi então reduzido:
para uma origem Android 720×1280, a janela abriu em 383×680. O usuário confirmou
que via a janela e clicou em HELLO. Logo depois, o log Android registrou
`AudioTrack: stop(8): called with 27400 frames delivered`, evidência de que o
toque chegou ao Native Audio e disparou a reprodução. O AudioFlinger reportou
zero underruns na inspeção, com rota Android `Speaker`; não se confirmou som
audível no host. Uma amostra anterior de `ps`, aos 2 min 46 s, marcou 68,2% de
CPU e 1,2% de memória; não é uma média. O viewer, container, volume, imagem e
endpoint ADB temporários foram encerrados e removidos.

Redroid é um projeto externo; Drunk não é cliente oficial do Roblox. Não houve
instalação ou execução de Roblox neste spike, e não há promessa de suporte.
O gate de arquitetura/licenças e as limitações documentadas em
[runtime-evaluation.md](runtime-evaluation.md) continuam aplicáveis. Nenhuma
checagem de integridade, attestation ou restrição de plataforma é alterada.
O probe foi executado contra o container redroid Android 12: recebeu 1.031.795
bytes em 5,27 s (0,19 MiB/s), primeiro byte em 386 ms, e encontrou NALs Annex-B
SPS, PPS e de imagem. Foi uma única captura da tela Android; não mede decode,
apresentação, latência ponta a ponta ou estabilidade prolongada. Container,
volume e imagem foram removidos depois da coleta.

## Correção do caminho de áudio com Android 13

O teste Android 12 acima mostrou por que um sink-input ativo não bastava como
prova: durante HELLO, o monitor HDMI capturou somente zeros. A fonte scrcpy
`output` (`REMOTE_SUBMIX`) foi, portanto, removida do controlador. `playback`
requer Android 13/API 33 ou superior; a imagem de teste foi fixada no digest
`redroid/redroid@sha256:36d6d21bcf7e92d78eabaa6f1748e5cf0e9fb15176c0091794168be012c5c22e`.

Com Native Audio HELLO tocando no Android 13 e scrcpy 3.3.4 executando
`--audio-source=playback`, uma captura de 3 s do monitor HDMI a 48 kHz registrou
98.304 amostras PCM intercaladas (2 canais), pico `640434624`, RMS `0,04817685`
e 48.026 amostras não nulas. Isso prova que o stream Android 13 produziu sinal na saída
HDMI durante o teste. O usuário confirmou ter ouvido o HELLO nos testes direto e
integrado. O resultado não prova compatibilidade com todos os apps, nem
qualidade/latência adequadas para jogos. O comando padrão do Drunk agora usa
Android 13 e `playback`; a integração final pelo próprio comando ainda precisa
ser repetida após a troca do digest.
