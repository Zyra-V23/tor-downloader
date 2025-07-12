# 🚀 Guía de Inicio Rápido

## ¿Qué es esto?

Una **suite completa de herramientas** para descargar sitios .onion de forma recursiva y multi-hilo:

### 📁 Archivos Principales:
- `onion_downloader.sh` - Script maestro (interfaz principal)
- `download_onion_multithreaded.sh` - Motor de descarga multi-hilo
- `config.sh` - Configurador interactivo
- `monitor.sh` - Monitor en tiempo real
- `README.md` - Documentación completa

## 🎯 Empezar en 30 Segundos

### Opción 1: Inicio Súper Rápido
```bash
./onion_downloader.sh quick-start
```

### Opción 2: Paso a Paso
```bash
# 1. Verificar dependencias
./onion_downloader.sh status

# 2. Configurar parámetros (incluye URL .onion)
./onion_downloader.sh config

# 3. Iniciar descarga
./onion_downloader.sh start

# 4. Monitorear progreso (en otra terminal)
./onion_downloader.sh monitor
```

## 🔧 Configuración Inicial

En la primera ejecución, el sistema te pedirá:
- **URL del sitio .onion**: La URL que quieres descargar
- **Directorio de descarga**: Donde guardar los archivos
- **Hilos de descarga**: Número de descargas simultáneas
- **Configuración de Tor**: Proxy y puertos

## 📊 Características Principales

### Multi-Threading Avanzado
- ✅ Hasta 20 descargas simultáneas
- ✅ Control de concurrencia con semáforos
- ✅ Balanceador automático de carga

### Monitoreo en Tiempo Real
- ✅ Estadísticas en vivo
- ✅ Velocidad de descarga
- ✅ Conteo de archivos/errores
- ✅ Estimación de tiempo

### Configuración Inteligente
- ✅ Validación de parámetros
- ✅ Prueba de conectividad
- ✅ Estimación de tamaño
- ✅ Configuración persistente

### Manejo de Errores
- ✅ Reintentos automáticos
- ✅ Logging detallado
- ✅ Limpieza automática
- ✅ Recuperación de fallos

## 🎮 Comandos Más Útiles

```bash
# Ver estado completo
./onion_downloader.sh status

# Configurar todo desde cero
./onion_downloader.sh config

# Iniciar descarga en background
./onion_downloader.sh start &

# Monitor en tiempo real
./onion_downloader.sh monitor

# Ver logs recientes
./onion_downloader.sh logs

# Parar todo
./onion_downloader.sh stop

# Limpiar archivos temporales
./onion_downloader.sh cleanup
```

## 🔍 Monitoreo Avanzado

```bash
# Monitor completo en tiempo real
./monitor.sh --realtime

# Solo estadísticas
./monitor.sh --stats

# Generar reporte detallado
./monitor.sh --report
```

## 🛠️ Solución Rápida de Problemas

### Si no funciona:
```bash
# 1. Verificar que Tor esté ejecutándose
./onion_downloader.sh tor-status

# 2. Iniciar Tor si es necesario
./onion_downloader.sh tor-start

# 3. Probar conectividad
./config.sh --test

# 4. Limpiar archivos temporales
./onion_downloader.sh cleanup
```

### Si es muy lento:
```bash
# Reconfigurar para reducir concurrencia
./onion_downloader.sh config
# Cambia MAX_CONCURRENT_DOWNLOADS a 4-6
# Aumenta DELAY_BETWEEN_REQUESTS a 2-3
```

## 📈 Optimización

### Para sitios pequeños (< 1000 archivos):
- **Hilos**: 8-12
- **Delay**: 1s
- **Timeout**: 30s

### Para sitios grandes (> 1000 archivos):
- **Hilos**: 6-8
- **Delay**: 2s
- **Timeout**: 45s

### Para conexiones lentas:
- **Hilos**: 4-6
- **Delay**: 3s
- **Timeout**: 60s

## 🔐 Seguridad

- **Tor obligatorio**: Todas las conexiones via SOCKS5
- **User Agent**: Simula navegador estándar
- **Timeouts**: Evita conexiones colgadas
- **Validación**: URLs .onion validadas antes de usar

## 📦 Requisitos

- **Windows 10+** con Git Bash
- **Tor Browser** o **Tor standalone**
- **curl** (incluido en Git Bash)

## 🎉 ¡Listo para Usar!

**Comando recomendado para empezar:**
```bash
./onion_downloader.sh quick-start
```

Este comando te guiará paso a paso y configurará todo automáticamente, incluyendo solicitar la URL .onion que quieres descargar.

---

**⚠️ Aviso Legal**: Esta herramienta está destinada para uso educativo y legal. El usuario es responsable de cumplir con todas las leyes locales y términos de servicio aplicables.

**🛡️ Seguridad**: Siempre usa Tor y mantén tu anonimato al acceder a sitios .onion. 