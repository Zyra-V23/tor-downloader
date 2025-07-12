# 🧅 Onion Site Downloader Suite

**Multi-threaded recursive downloader for .onion sites with advanced monitoring and configuration**

## 📋 Tabla de Contenidos

- [Características](#características)
- [Requisitos](#requisitos)
- [Instalación](#instalación)
- [Uso Rápido](#uso-rápido)
- [Componentes](#componentes)
- [Configuración](#configuración)
- [Comandos](#comandos)
- [Monitoreo](#monitoreo)
- [Solución de Problemas](#solución-de-problemas)
- [Notas Técnicas](#notas-técnicas)

## ✨ Características

- **Multi-threading**: Hasta 20 descargas concurrentes
- **Recursivo**: Descarga automática de directorios y subdirectorios
- **Reintentos**: Sistema de reintentos automáticos con backoff
- **Monitoreo**: Monitor en tiempo real con estadísticas
- **Configuración**: Sistema de configuración interactiva
- **Logs**: Logging detallado con separación de errores
- **Tor Integration**: Manejo automático de Tor
- **Windows Compatible**: Optimizado para Windows con Git Bash

## 🔧 Requisitos

### Software Necesario
- **Windows 10+** con Git Bash
- **Tor Browser** o **Tor standalone**
- **curl** (incluido en Git Bash)
- **Chocolatey** (para instalación de dependencias)

### Dependencias Automáticas
```bash
# Si no tienes Tor instalado
choco install tor -y

# Si no tienes wget (opcional)
choco install wget -y
```

## 🚀 Instalación

1. **Descargar los archivos**:
   ```bash
   # Clona o descarga todos los archivos .sh en tu directorio de trabajo
   cd "C:/Users/TuUsuario/Desktop/sitiojulian"
   ```

2. **Hacer ejecutables** (si es necesario):
   ```bash
   chmod +x *.sh
   ```

3. **Verificar instalación**:
   ```bash
   ./onion_downloader.sh status
   ```

## 🎯 Uso Rápido

### Modo Quick Start (Recomendado)
```bash
./onion_downloader.sh quick-start
```

Este modo te guiará paso a paso:
1. Verifica dependencias
2. Inicia Tor automáticamente
3. Configura parámetros interactivamente
4. Inicia la descarga
5. Abre el monitor en tiempo real

### Uso Básico
```bash
# Iniciar descarga con configuración existente
./onion_downloader.sh start

# Monitorear progreso
./onion_downloader.sh monitor

# Ver estado actual
./onion_downloader.sh status

# Parar descarga
./onion_downloader.sh stop
```

## 📦 Componentes

### 1. Script Principal (`onion_downloader.sh`)
- **Función**: Interfaz principal y menú interactivo
- **Uso**: `./onion_downloader.sh [comando]`

### 2. Descargador Multi-hilo (`download_onion_multithreaded.sh`)
- **Función**: Motor de descarga recursiva con multi-threading
- **Características**:
  - Semáforos para control de concurrencia
  - Parseo HTML inteligente
  - Manejo de errores y reintentos
  - Limpieza de nombres de archivo para Windows

### 3. Configurador (`config.sh`)
- **Función**: Configuración interactiva de parámetros
- **Características**:
  - Validación de entrada
  - Valores por defecto
  - Prueba de conectividad
  - Estimación de descarga

### 4. Monitor (`monitor.sh`)
- **Función**: Monitoreo en tiempo real del progreso
- **Características**:
  - Estadísticas en tiempo real
  - Estimación de velocidad
  - Reporte de actividad reciente
  - Generación de reportes

## ⚙️ Configuración

### Configuración Interactiva
```bash
./config.sh
```

### Parámetros Configurables
- **URL del sitio .onion**
- **Proxy de Tor** (IP:Puerto)
- **Descargas concurrentes** (1-20)
- **Directorio de descarga**
- **Intentos de reintento** (1-10)
- **Delay entre requests** (0-10s)
- **Timeout de requests** (10-300s)
- **User Agent string**

### Archivo de Configuración
Los parámetros se guardan en `download_config.conf`:
```bash
# Ejemplo de configuración
ONION_URL="http://your-target-site.onion"
TOR_PROXY="127.0.0.1:9050"
MAX_CONCURRENT_DOWNLOADS=8
DOWNLOAD_DIR="./downloaded_site"
RETRY_ATTEMPTS=3
DELAY_BETWEEN_REQUESTS=1
TIMEOUT=30
```

## 🎮 Comandos

### Script Principal
```bash
./onion_downloader.sh [comando]
```

**Comandos disponibles**:
- `start` - Iniciar descarga
- `stop` - Parar descarga
- `monitor` - Monitorear progreso
- `config` - Configurar parámetros
- `status` - Ver estado actual
- `logs` - Ver logs recientes
- `tor-start` - Iniciar Tor
- `tor-stop` - Parar Tor
- `tor-status` - Estado de Tor
- `cleanup` - Limpiar archivos temporales
- `quick-start` - Inicio rápido guiado
- `help` - Mostrar ayuda

### Scripts Individuales
```bash
# Configuración
./config.sh --interactive    # Modo interactivo
./config.sh --test          # Probar conexión
./config.sh --show          # Mostrar configuración

# Monitor
./monitor.sh --realtime     # Monitor en tiempo real
./monitor.sh --stats        # Estadísticas actuales
./monitor.sh --report       # Generar reporte
```

## 📊 Monitoreo

### Monitor en Tiempo Real
```bash
./monitor.sh --realtime
```

**Muestra**:
- Archivos descargados
- Tamaño total
- Descargas exitosas/errores
- Descargas activas
- Velocidad estimada
- Actividad reciente

### Estadísticas Rápidas
```bash
./monitor.sh --stats
```

### Reportes Detallados
```bash
./monitor.sh --report
```

Genera un archivo `download_report_YYYYMMDD_HHMMSS.txt` con:
- Resumen estadístico
- Estructura de directorios
- Archivos más grandes
- Logs detallados
- Errores encontrados

## 🔍 Logs

### Archivos de Log
- `download.log` - Log principal con toda la actividad
- `error.log` - Log de errores específicos

### Ver Logs
```bash
# Logs recientes
./onion_downloader.sh logs

# Logs en tiempo real
tail -f download.log
tail -f error.log
```

## 🛠️ Solución de Problemas

### Problemas Comunes

#### 1. "Tor is not running"
```bash
# Iniciar Tor manualmente
./onion_downloader.sh tor-start

# O desde línea de comandos
tor --quiet --daemon
```

#### 2. "Cannot connect to .onion URL"
- Verifica que Tor esté ejecutándose
- Verifica que la URL .onion sea correcta
- Prueba la conectividad: `./config.sh --test`

#### 3. "Permission denied"
```bash
# Hacer ejecutables todos los scripts
chmod +x *.sh
```

#### 4. Downloads muy lentos
- Reduce `MAX_CONCURRENT_DOWNLOADS` a 4-6
- Aumenta `DELAY_BETWEEN_REQUESTS` a 2-3
- Verifica tu conexión a Tor

#### 5. "Semaphore errors"
```bash
# Limpiar archivos temporales
./onion_downloader.sh cleanup
```

### Debugging

#### Modo Verbose
```bash
# Activar logging detallado
export DEBUG=1
./download_onion_multithreaded.sh
```

#### Verificar Procesos
```bash
# Ver procesos activos
ps aux | grep curl
ps aux | grep tor

# Parar todos los procesos
./onion_downloader.sh stop
```

## 🔧 Notas Técnicas

### Arquitectura
- **Semáforos**: Control de concurrencia usando archivos temporales
- **Background Jobs**: Descargas paralelas con `&`
- **Signal Handling**: Limpieza automática con `trap`
- **URL Parsing**: Regex para extraer enlaces de HTML

### Optimizaciones
- **Timeout escalonado**: Timeout más largo para archivos grandes
- **Reintentos con backoff**: Espera progresiva entre reintentos
- **Limpieza de nombres**: Caracteres problemáticos para Windows
- **Decodificación URL**: Soporte para URLs codificadas

### Limitaciones
- **Profundidad máxima**: 20 niveles de recursión
- **Concurrencia máxima**: 20 descargas simultáneas
- **Tipos de archivo**: Solo HTTP/HTTPS links
- **Memoria**: Dependiente del número de archivos

### Seguridad
- **Tor obligatorio**: Todas las conexiones via SOCKS5
- **User Agent**: Simula navegador estándar
- **No ejecutables**: Solo descarga archivos
- **Timeouts**: Evita conexiones colgadas

## 📈 Rendimiento

### Configuración Recomendada
```bash
# Para sitios pequeños (<1000 archivos)
MAX_CONCURRENT_DOWNLOADS=8
DELAY_BETWEEN_REQUESTS=1

# Para sitios grandes (>1000 archivos)
MAX_CONCURRENT_DOWNLOADS=6
DELAY_BETWEEN_REQUESTS=2

# Para conexiones lentas
MAX_CONCURRENT_DOWNLOADS=4
DELAY_BETWEEN_REQUESTS=3
```

### Monitoreo de Recursos
```bash
# Monitorear uso de memoria
ps aux | grep curl | wc -l

# Monitorear conexiones de red
netstat -an | grep 9050

# Monitorear espacio en disco
df -h .
```

## 🤝 Contribución

### Mejoras Posibles
- [ ] Interfaz GUI
- [ ] Soporte para otros tipos de proxy
- [ ] Filtros de tipo de archivo
- [ ] Compresión automática
- [ ] Soporte para autenticación
- [ ] Base de datos de URLs visitadas

### Reportar Bugs
1. Incluye los logs completos
2. Especifica la configuración usada
3. Describe los pasos para reproducir
4. Incluye información del sistema

## 📝 Licencia

Este proyecto es de código abierto y está disponible bajo la licencia MIT.

## 🔗 Enlaces Útiles

- [Tor Project](https://www.torproject.org/)
- [Chocolatey](https://chocolatey.org/)
- [Git for Windows](https://git-scm.com/download/win)
- [curl Documentation](https://curl.se/docs/)

---

**⚠️ Aviso Legal**: Esta herramienta está destinada para uso educativo y legal. El usuario es responsable de cumplir con todas las leyes locales y términos de servicio aplicables.

**🛡️ Seguridad**: Siempre usa Tor y mantén tu anonimato al acceder a sitios .onion. 