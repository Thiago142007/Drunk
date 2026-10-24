# Spike redroid controlado pelo Drunk

Data: 2026-10-24. Este protótipo adiciona um controlador Rust pequeno para um
Android externo. O código próprio do Drunk usa Rust, `minifb` e `image` para
exibir screenshots e encaminhar eventos básicos; Docker, ADB e a imagem Android
são componentes externos e não são inteiramente Rust.
O **NO-GO de produto permanece**: esta execução não seleciona o backend final,
resolve o inventário de licenças nem demonstra compatibilidade com Roblox.

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
target/debug/drunk redroid stop
```

- `start` exige `--allow-privileged`, verifica `/proc/modules` e `docker info` e
  cria o container com os parâmetros fixos acima. Um container já em execução
  com a mesma imagem, modo privilegiado bridge, ADB somente em loopback, política sem
  reinicialização e volume nomeado retorna sucesso sem reiniciá-lo.
  `demo` também exige essa configuração. Se o nome existir com outra imagem ou estado, a CLI pede inspeção manual;
  não remove nem substitui o container. O sucesso de `start` informa que Docker
  iniciou o container; não garante que Android já terminou o boot.
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
  540×900. Um clique esquerdo envia `input tap`; teclas básicas (letras, números,
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
  janela `minifb`. O limite selecionável é 1–180 segundos (30 por padrão), pois
  `screenrecord` tem duração máxima finita. Escape fecha a janela e o processo
  local ADB é encerrado e recolhido. O decoder fica em uma thread separada; um
  panic do Hibernia vira erro compreensível. A fila de frames tem capacidade um
  e substitui frames antigos para não acumular atraso se a janela não acompanhar
  o decoder. Esta é uma experiência de visualização; o comando não implementa
  entrada de jogo nem áudio.
- `stop` valida a configuração, para `drunk-redroid` e consulta o código de saída
  do container. Se Docker precisou enviar SIGKILL após o timeout, a CLI informa
  isso explicitamente; no redroid 12 medido aqui, `Exited (137)` ocorreu mesmo
  com timeout de 60 segundos. O volume de dados e a imagem permanecem, assim como
  o objeto Docker parado. Para evitar reutilização silenciosa, um `start` posterior
  encontra esse objeto e exige inspeção manual. A CLI não descarrega Binder nem
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

Em uma repetição posterior autorizada, `redroid stream --seconds 30` também abriu
a janela 720×1280 e permaneceu até o limite configurado. Uma amostra de processo
aos 28 s mostrou o Drunk em aproximadamente 92% de CPU e 1,3% de memória, com
`screenrecord` em 0,4% de CPU. É apenas uma amostra, sem média de CPU, FPS ou
latência; o conteúdo da janela não foi inspecionado independentemente. O estado
Docker após `drunk redroid stop` foi `Exited (137)`. `docker container stop --time
60` também deixou `Exited (137)` após 62,37 s. A imagem usa `/init` como entrypoint
e não define `StopSignal`; `adb shell reboot -p` retornou `Done`, mas o container
permaneceu em execução. A causa exata não foi confirmada; parada graciosa não foi
demonstrada. Container, volume, imagem e endpoint ADB temporários foram removidos.

Também foram compilados e lançados os samples oficiais `Hello GL2` e `Native
Audio` do Android NDK Samples. GL2 desenhou o triângulo e informou ANGLE sobre
Vulkan SwiftShader; isso é renderização por software. O botão local HELLO do
Native Audio criou um track no AudioFlinger com rota Android `speaker`, sem
underruns na amostra. Não se confirmou áudio audível no host e a permissão
`RECORD_AUDIO` ficou negada. Cada app permaneceu aberto por 60 s e foi encerrado
e relançado duas vezes sem `FATAL EXCEPTION` nos logs consultados. A captura da
tela não mede áudio físico, input físico nem estabilidade prolongada do viewer.

Não foi validado Roblox, áudio audível no host, aceleração gráfica física ou
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

O viewer H.264 abriu uma janela por 30 s, mas a própria janela não foi
inspecionada visualmente de forma independente; uma amostra do processo marcou
aproximadamente 92% de CPU. Hello GL2 e Native Audio foram inspecionados por
capturas ADB; o encaminhamento de toque/teclas pela janela Drunk não foi testado
com input físico. O track de áudio Android não comprova som audível no host. A
renderização observada foi SwiftShader software. O spike não inclui espera de
boot, recuperação automática, atualização ou distribuição de imagens Android.

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
