"""
Indicates exactly one field must be supplied and this field must not be `null`.
"""
directive @oneOf on INPUT_OBJECT

"""Directive to document required permissions for fields"""
directive @usePermissions(
  """The action verb required for access"""
  action: AuthActionVerb

  """The resource required for access"""
  resource: String

  """The possession type required for access"""
  possession: AuthPossession
) on FIELD_DEFINITION

type ApiKeyResponse {
  valid: Boolean!
  error: String
}

type MinigraphqlResponse {
  status: MinigraphStatus!
  timeout: Int
  error: String
}

enum MinigraphStatus {
  PRE_INIT
  CONNECTING
  CONNECTED
  PING_FAILURE
  ERROR_RETRYING
}

type CloudResponse {
  status: String!
  ip: String
  error: String
}

type RelayResponse {
  status: String!
  timeout: String
  error: String
}

type Cloud {
  error: String
  apiKey: ApiKeyResponse!
  relay: RelayResponse
  minigraphql: MinigraphqlResponse!
  cloud: CloudResponse!
  allowedOrigins: [String!]!
}

type Capacity {
  """Free capacity"""
  free: String!

  """Used capacity"""
  used: String!

  """Total capacity"""
  total: String!
}

type ArrayCapacity {
  """Capacity in kilobytes"""
  kilobytes: Capacity!

  """Capacity in number of disks"""
  disks: Capacity!
}

type ArrayDisk implements Node {
  id: PrefixedID!

  """
  Array slot number. Parity1 is always 0 and Parity2 is always 29. Array slots will be 1 - 28. Cache slots are 30 - 53. Flash is 54.
  """
  idx: Int!
  name: String
  device: String

  """(KB) Disk Size total"""
  size: Long
  status: ArrayDiskStatus

  """Is the disk a HDD or SSD."""
  rotational: Boolean

  """Disk temp - will be NaN if array is not started or DISK_NP"""
  temp: Int

  """
  Count of I/O read requests sent to the device I/O drivers. These statistics may be cleared at any time.
  """
  numReads: Long

  """
  Count of I/O writes requests sent to the device I/O drivers. These statistics may be cleared at any time.
  """
  numWrites: Long

  """
  Number of unrecoverable errors reported by the device I/O drivers. Missing data due to unrecoverable array read errors is filled in on-the-fly using parity reconstruct (and we attempt to write this data back to the sector(s) which failed). Any unrecoverable write error results in disabling the disk.
  """
  numErrors: Long

  """(KB) Total Size of the FS (Not present on Parity type drive)"""
  fsSize: Long

  """(KB) Free Size on the FS (Not present on Parity type drive)"""
  fsFree: Long

  """(KB) Used Size on the FS (Not present on Parity type drive)"""
  fsUsed: Long
  exportable: Boolean

  """Type of Disk - used to differentiate Cache / Flash / Array / Parity"""
  type: ArrayDiskType!

  """(%) Disk space left to warn"""
  warning: Int

  """(%) Disk space left for critical"""
  critical: Int

  """File system type for the disk"""
  fsType: String

  """User comment on disk"""
  comment: String

  """File format (ex MBR: 4KiB-aligned)"""
  format: String

  """ata | nvme | usb | (others)"""
  transport: String
  color: ArrayDiskFsColor
}

interface Node {
  id: PrefixedID!
}

"""The `Long` scalar type represents 52-bit integers"""
scalar Long

enum ArrayDiskStatus {
  DISK_NP
  DISK_OK
  DISK_NP_MISSING
  DISK_INVALID
  DISK_WRONG
  DISK_DSBL
  DISK_NP_DSBL
  DISK_DSBL_NEW
  DISK_NEW
}

enum ArrayDiskType {
  DATA
  PARITY
  FLASH
  CACHE
}

enum ArrayDiskFsColor {
  GREEN_ON
  GREEN_BLINK
  BLUE_ON
  BLUE_BLINK
  YELLOW_ON
  YELLOW_BLINK
  RED_ON
  RED_OFF
  GREY_OFF
}

type UnraidArray implements Node {
  id: PrefixedID!

  """Current array state"""
  state: ArrayState!

  """Current array capacity"""
  capacity: ArrayCapacity!

  """Current boot disk"""
  boot: ArrayDisk

  """Parity disks in the current array"""
  parities: [ArrayDisk!]!

  """Data disks in the current array"""
  disks: [ArrayDisk!]!

  """Caches in the current array"""
  caches: [ArrayDisk!]!
}

enum ArrayState {
  STARTED
  STOPPED
  NEW_ARRAY
  RECON_DISK
  DISABLE_DISK
  SWAP_DSBL
  INVALID_EXPANSION
  PARITY_NOT_BIGGEST
  TOO_MANY_MISSING_DISKS
  NEW_DISK_TOO_SMALL
  NO_DATA_DISKS
}

type Share implements Node {
  id: PrefixedID!

  """Display name"""
  name: String

  """(KB) Free space"""
  free: Long

  """(KB) Used Size"""
  used: Long

  """(KB) Total size"""
  size: Long

  """Disks that are included in this share"""
  include: [String!]

  """Disks that are excluded from this share"""
  exclude: [String!]

  """Is this share cached"""
  cache: Boolean

  """Original name"""
  nameOrig: String

  """User comment"""
  comment: String

  """Allocator"""
  allocator: String

  """Split level"""
  splitLevel: String

  """Floor"""
  floor: String

  """COW"""
  cow: String

  """Color"""
  color: String

  """LUKS status"""
  luksStatus: String
}

type AccessUrl {
  type: URL_TYPE!
  name: String
  ipv4: URL
  ipv6: URL
}

enum URL_TYPE {
  LAN
  WIREGUARD
  WAN
  MDNS
  OTHER
  DEFAULT
}

"""
A field whose value conforms to the standard URL format as specified in RFC3986: https://www.ietf.org/rfc/rfc3986.txt.
"""
scalar URL

type RemoteAccess {
  """The type of WAN access used for Remote Access"""
  accessType: WAN_ACCESS_TYPE!

  """The type of port forwarding used for Remote Access"""
  forwardType: WAN_FORWARD_TYPE

  """The port used for Remote Access"""
  port: Int
}

enum WAN_ACCESS_TYPE {
  DYNAMIC
  ALWAYS
  DISABLED
}

enum WAN_FORWARD_TYPE {
  UPNP
  STATIC
}

type DynamicRemoteAccessStatus {
  """The type of dynamic remote access that is enabled"""
  enabledType: DynamicRemoteAccessType!

  """The type of dynamic remote access that is currently running"""
  runningType: DynamicRemoteAccessType!

  """Any error message associated with the dynamic remote access"""
  error: String
}

enum DynamicRemoteAccessType {
  STATIC
  UPNP
  DISABLED
}

type ConnectSettingsValues {
  """
  If true, the GraphQL sandbox is enabled and available at /graphql. If false, the GraphQL sandbox is disabled and only the production API will be available.
  """
  sandbox: Boolean!

  """A list of origins allowed to interact with the API"""
  extraOrigins: [String!]!

  """The type of WAN access used for Remote Access"""
  accessType: WAN_ACCESS_TYPE!

  """The type of port forwarding used for Remote Access"""
  forwardType: WAN_FORWARD_TYPE

  """The port used for Remote Access"""
  port: Int

  """A list of Unique Unraid Account ID's"""
  ssoUserIds: [String!]!
}

type ConnectSettings implements Node {
  id: PrefixedID!

  """The data schema for the Connect settings"""
  dataSchema: JSON!

  """The UI schema for the Connect settings"""
  uiSchema: JSON!

  """The values for the Connect settings"""
  values: ConnectSettingsValues!
}

"""
The `JSON` scalar type represents JSON values as specified by [ECMA-404](http://www.ecma-international.org/publications/files/ECMA-ST/ECMA-404.pdf).
"""
scalar JSON

type Connect implements Node {
  id: PrefixedID!

  """The status of dynamic remote access"""
  dynamicRemoteAccess: DynamicRemoteAccessStatus!

  """The settings for the Connect instance"""
  settings: ConnectSettings!
}

type Network implements Node {
  id: PrefixedID!
  accessUrls: [AccessUrl!]
}

type ProfileModel implements Node {
  id: PrefixedID!
  username: String!
  url: String!
  avatar: String!
}

type Server implements Node {
  id: PrefixedID!
  owner: ProfileModel!
  guid: String!
  apikey: String!
  name: String!
  status: ServerStatus!
  wanip: String!
  lanip: String!
  localurl: String!
  remoteurl: String!
}

enum ServerStatus {
  ONLINE
  OFFLINE
  NEVER_CONNECTED
}

type DiskPartition {
  """The name of the partition"""
  name: String!

  """The filesystem type of the partition"""
  fsType: DiskFsType!

  """The size of the partition in bytes"""
  size: Float!
}

"""The type of filesystem on the disk partition"""
enum DiskFsType {
  XFS
  BTRFS
  VFAT
  ZFS
  EXT4
  NTFS
}

type Disk implements Node {
  id: PrefixedID!

  """The device path of the disk (e.g. /dev/sdb)"""
  device: String!

  """The type of disk (e.g. SSD, HDD)"""
  type: String!

  """The model name of the disk"""
  name: String!

  """The manufacturer of the disk"""
  vendor: String!

  """The total size of the disk in bytes"""
  size: Float!

  """The number of bytes per sector"""
  bytesPerSector: Float!

  """The total number of cylinders on the disk"""
  totalCylinders: Float!

  """The total number of heads on the disk"""
  totalHeads: Float!

  """The total number of sectors on the disk"""
  totalSectors: Float!

  """The total number of tracks on the disk"""
  totalTracks: Float!

  """The number of tracks per cylinder"""
  tracksPerCylinder: Float!

  """The number of sectors per track"""
  sectorsPerTrack: Float!

  """The firmware revision of the disk"""
  firmwareRevision: String!

  """The serial number of the disk"""
  serialNum: String!

  """The interface type of the disk"""
  interfaceType: DiskInterfaceType!

  """The SMART status of the disk"""
  smartStatus: DiskSmartStatus!

  """The current temperature of the disk in Celsius"""
  temperature: Float

  """The partitions on the disk"""
  partitions: [DiskPartition!]!
}

"""The type of interface the disk uses to connect to the system"""
enum DiskInterfaceType {
  SAS
  SATA
  USB
  PCIE
  UNKNOWN
}

"""
The SMART (Self-Monitoring, Analysis and Reporting Technology) status of the disk
"""
enum DiskSmartStatus {
  OK
  UNKNOWN
}

type KeyFile {
  location: String
  contents: String
}

type Registration implements Node {
  id: PrefixedID!
  type: registrationType
  keyFile: KeyFile
  state: RegistrationState
  expiration: String
  updateExpiration: String
}

enum registrationType {
  BASIC
  PLUS
  PRO
  STARTER
  UNLEASHED
  LIFETIME
  INVALID
  TRIAL
}

enum RegistrationState {
  TRIAL
  BASIC
  PLUS
  PRO
  STARTER
  UNLEASHED
  LIFETIME
  EEXPIRED
  EGUID
  EGUID1
  ETRIAL
  ENOKEYFILE
  ENOKEYFILE1
  ENOKEYFILE2
  ENOFLASH
  ENOFLASH1
  ENOFLASH2
  ENOFLASH3
  ENOFLASH4
  ENOFLASH5
  ENOFLASH6
  ENOFLASH7
  EBLACKLISTED
  EBLACKLISTED1
  EBLACKLISTED2
  ENOCONN
}

type Vars implements Node {
  id: PrefixedID!

  """Unraid version"""
  version: String
  maxArraysz: Int
  maxCachesz: Int

  """Machine hostname"""
  name: String
  timeZone: String
  comment: String
  security: String
  workgroup: String
  domain: String
  domainShort: String
  hideDotFiles: Boolean
  localMaster: Boolean
  enableFruit: String

  """Should a NTP server be used for time sync?"""
  useNtp: Boolean

  """NTP Server 1"""
  ntpServer1: String

  """NTP Server 2"""
  ntpServer2: String

  """NTP Server 3"""
  ntpServer3: String

  """NTP Server 4"""
  ntpServer4: String
  domainLogin: String
  sysModel: String
  sysArraySlots: Int
  sysCacheSlots: Int
  sysFlashSlots: Int
  useSsl: Boolean

  """Port for the webui via HTTP"""
  port: Int

  """Port for the webui via HTTPS"""
  portssl: Int
  localTld: String
  bindMgt: Boolean

  """Should telnet be enabled?"""
  useTelnet: Boolean
  porttelnet: Int
  useSsh: Boolean
  portssh: Int
  startPage: String
  startArray: Boolean
  spindownDelay: String
  queueDepth: String
  spinupGroups: Boolean
  defaultFormat: String
  defaultFsType: String
  shutdownTimeout: Int
  luksKeyfile: String
  pollAttributes: String
  pollAttributesDefault: String
  pollAttributesStatus: String
  nrRequests: Int
  nrRequestsDefault: Int
  nrRequestsStatus: String
  mdNumStripes: Int
  mdNumStripesDefault: Int
  mdNumStripesStatus: String
  mdSyncWindow: Int
  mdSyncWindowDefault: Int
  mdSyncWindowStatus: String
  mdSyncThresh: Int
  mdSyncThreshDefault: Int
  mdSyncThreshStatus: String
  mdWriteMethod: Int
  mdWriteMethodDefault: String
  mdWriteMethodStatus: String
  shareDisk: String
  shareUser: String
  shareUserInclude: String
  shareUserExclude: String
  shareSmbEnabled: Boolean
  shareNfsEnabled: Boolean
  shareAfpEnabled: Boolean
  shareInitialOwner: String
  shareInitialGroup: String
  shareCacheEnabled: Boolean
  shareCacheFloor: String
  shareMoverSchedule: String
  shareMoverLogging: Boolean
  fuseRemember: String
  fuseRememberDefault: String
  fuseRememberStatus: String
  fuseDirectio: String
  fuseDirectioDefault: String
  fuseDirectioStatus: String
  shareAvahiEnabled: Boolean
  shareAvahiSmbName: String
  shareAvahiSmbModel: String
  shareAvahiAfpName: String
  shareAvahiAfpModel: String
  safeMode: Boolean
  startMode: String
  configValid: Boolean
  configError: ConfigErrorState
  joinStatus: String
  deviceCount: Int
  flashGuid: String
  flashProduct: String
  flashVendor: String
  regCheck: String
  regFile: String
  regGuid: String
  regTy: registrationType
  regState: RegistrationState

  """Registration owner"""
  regTo: String
  regTm: String
  regTm2: String
  regGen: String
  sbName: String
  sbVersion: String
  sbUpdated: String
  sbEvents: Int
  sbState: String
  sbClean: Boolean
  sbSynced: Int
  sbSyncErrs: Int
  sbSynced2: Int
  sbSyncExit: String
  sbNumDisks: Int
  mdColor: String
  mdNumDisks: Int
  mdNumDisabled: Int
  mdNumInvalid: Int
  mdNumMissing: Int
  mdNumNew: Int
  mdNumErased: Int
  mdResync: Int
  mdResyncCorr: String
  mdResyncPos: String
  mdResyncDb: String
  mdResyncDt: String
  mdResyncAction: String
  mdResyncSize: Int
  mdState: String
  mdVersion: String
  cacheNumDevices: Int
  cacheSbNumDisks: Int
  fsState: String

  """Human friendly string of array events happening"""
  fsProgress: String

  """
  Percentage from 0 - 100 while upgrading a disk or swapping parity drives
  """
  fsCopyPrcnt: Int
  fsNumMounted: Int
  fsNumUnmountable: Int
  fsUnmountableMask: String

  """Total amount of user shares"""
  shareCount: Int

  """Total amount shares with SMB enabled"""
  shareSmbCount: Int

  """Total amount shares with NFS enabled"""
  shareNfsCount: Int

  """Total amount shares with AFP enabled"""
  shareAfpCount: Int
  shareMoverActive: Boolean
  csrfToken: String
}

"""Possible error states for configuration"""
enum ConfigErrorState {
  UNKNOWN_ERROR
  INELIGIBLE
  INVALID
  NO_KEY_SERVER
  WITHDRAWN
}

type Permission {
  resource: Resource!
  actions: [String!]!
}

"""Available resources for permissions"""
enum Resource {
  ACTIVATION_CODE
  API_KEY
  ARRAY
  CLOUD
  CONFIG
  CONNECT
  CONNECT__REMOTE_ACCESS
  CUSTOMIZATIONS
  DASHBOARD
  DISK
  DISPLAY
  DOCKER
  FLASH
  INFO
  LOGS
  ME
  NETWORK
  NOTIFICATIONS
  ONLINE
  OS
  OWNER
  PERMISSION
  REGISTRATION
  SERVERS
  SERVICES
  SHARE
  VARS
  VMS
  WELCOME
}

type ApiKey implements Node {
  id: PrefixedID!
  name: String!
  description: String
  roles: [Role!]!
  createdAt: String!
  permissions: [Permission!]!
}

"""Available roles for API keys and users"""
enum Role {
  ADMIN
  CONNECT
  GUEST
}

type ApiKeyWithSecret implements Node {
  id: PrefixedID!
  name: String!
  description: String
  roles: [Role!]!
  createdAt: String!
  permissions: [Permission!]!
  key: String!
}

type ArrayMutations {
  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **ARRAY**
  - Possession: **ANY**
  
  #### Description:
  
  Set array state
  """
  setState(input: ArrayStateInput!): UnraidArray!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **ARRAY**
  - Possession: **ANY**
  
  #### Description:
  
  Add new disk to array
  """
  addDiskToArray(input: ArrayDiskInput!): UnraidArray!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **ARRAY**
  - Possession: **ANY**
  
  #### Description:
  
  Remove existing disk from array. NOTE: The array must be stopped before running this otherwise it'll throw an error.
  """
  removeDiskFromArray(input: ArrayDiskInput!): UnraidArray!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **ARRAY**
  - Possession: **ANY**
  
  #### Description:
  
  Mount a disk in the array
  """
  mountArrayDisk(id: PrefixedID!): ArrayDisk!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **ARRAY**
  - Possession: **ANY**
  
  #### Description:
  
  Unmount a disk from the array
  """
  unmountArrayDisk(id: PrefixedID!): ArrayDisk!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **ARRAY**
  - Possession: **ANY**
  
  #### Description:
  
  Clear statistics for a disk in the array
  """
  clearArrayDiskStatistics(id: PrefixedID!): Boolean!
}

input ArrayStateInput {
  """Array state"""
  desiredState: ArrayStateInputState!
}

enum ArrayStateInputState {
  START
  STOP
}

input ArrayDiskInput {
  """Disk ID"""
  id: PrefixedID!

  """The slot for the disk"""
  slot: Int
}

type DockerMutations {
  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **DOCKER**
  - Possession: **ANY**
  
  #### Description:
  
  Start a container
  """
  start(id: PrefixedID!): DockerContainer!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **DOCKER**
  - Possession: **ANY**
  
  #### Description:
  
  Stop a container
  """
  stop(id: PrefixedID!): DockerContainer!
}

type VmMutations {
  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **VMS**
  - Possession: **ANY**
  
  #### Description:
  
  Start a virtual machine
  """
  start(id: PrefixedID!): Boolean!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **VMS**
  - Possession: **ANY**
  
  #### Description:
  
  Stop a virtual machine
  """
  stop(id: PrefixedID!): Boolean!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **VMS**
  - Possession: **ANY**
  
  #### Description:
  
  Pause a virtual machine
  """
  pause(id: PrefixedID!): Boolean!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **VMS**
  - Possession: **ANY**
  
  #### Description:
  
  Resume a virtual machine
  """
  resume(id: PrefixedID!): Boolean!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **VMS**
  - Possession: **ANY**
  
  #### Description:
  
  Force stop a virtual machine
  """
  forceStop(id: PrefixedID!): Boolean!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **VMS**
  - Possession: **ANY**
  
  #### Description:
  
  Reboot a virtual machine
  """
  reboot(id: PrefixedID!): Boolean!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **VMS**
  - Possession: **ANY**
  
  #### Description:
  
  Reset a virtual machine
  """
  reset(id: PrefixedID!): Boolean!
}

"""
Parity check related mutations, WIP, response types and functionaliy will change
"""
type ParityCheckMutations {
  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **ARRAY**
  - Possession: **ANY**
  
  #### Description:
  
  Start a parity check
  """
  start(correct: Boolean!): JSON!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **ARRAY**
  - Possession: **ANY**
  
  #### Description:
  
  Pause a parity check
  """
  pause: JSON!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **ARRAY**
  - Possession: **ANY**
  
  #### Description:
  
  Resume a parity check
  """
  resume: JSON!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **ARRAY**
  - Possession: **ANY**
  
  #### Description:
  
  Cancel a parity check
  """
  cancel: JSON!
}

type ParityCheck {
  """Date of the parity check"""
  date: DateTime

  """Duration of the parity check in seconds"""
  duration: Int

  """Speed of the parity check, in MB/s"""
  speed: String

  """Status of the parity check"""
  status: String

  """Number of errors during the parity check"""
  errors: Int

  """Progress percentage of the parity check"""
  progress: Int

  """Whether corrections are being written to parity"""
  correcting: Boolean

  """Whether the parity check is paused"""
  paused: Boolean

  """Whether the parity check is running"""
  running: Boolean
}

"""
A date-time string at UTC, such as 2019-12-03T09:54:33Z, compliant with the date-time format.
"""
scalar DateTime

type Config implements Node {
  id: PrefixedID!
  valid: Boolean
  error: String
}

type PublicPartnerInfo {
  partnerName: String

  """Indicates if a partner logo exists"""
  hasPartnerLogo: Boolean!
  partnerUrl: String

  """
  The path to the partner logo image on the flash drive, relative to the activation code file
  """
  partnerLogoUrl: String
}

type ActivationCode {
  code: String
  partnerName: String
  partnerUrl: String
  serverName: String
  sysModel: String
  comment: String
  header: String
  headermetacolor: String
  background: String
  showBannerGradient: Boolean
  theme: String
}

type Customization {
  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **ACTIVATION_CODE**
  - Possession: **ANY**
  """
  activationCode: ActivationCode
  partnerInfo: PublicPartnerInfo
  theme: Theme!
}

type Theme {
  """The theme name"""
  name: ThemeName!

  """Whether to show the header banner image"""
  showBannerImage: Boolean!

  """Whether to show the banner gradient"""
  showBannerGradient: Boolean!

  """The background color of the header"""
  headerBackgroundColor: String!

  """Whether to show the description in the header"""
  showHeaderDescription: Boolean!

  """The text color of the header"""
  headerPrimaryTextColor: String!

  """The secondary text color of the header"""
  headerSecondaryTextColor: String
}

"""The theme name"""
enum ThemeName {
  azure
  black
  gray
  white
}

type InfoApps implements Node {
  id: PrefixedID!

  """How many docker containers are installed"""
  installed: Int!

  """How many docker containers are running"""
  started: Int!
}

type Baseboard implements Node {
  id: PrefixedID!
  manufacturer: String!
  model: String
  version: String
  serial: String
  assetTag: String
}

type InfoCpu implements Node {
  id: PrefixedID!
  manufacturer: String!
  brand: String!
  vendor: String!
  family: String!
  model: String!
  stepping: Int!
  revision: String!
  voltage: String
  speed: Float!
  speedmin: Float!
  speedmax: Float!
  threads: Int!
  cores: Int!
  processors: Int!
  socket: String!
  cache: JSON!
  flags: [String!]!
}

type Gpu implements Node {
  id: PrefixedID!
  type: String!
  typeid: String!
  vendorname: String!
  productid: String!
  blacklisted: Boolean!
  class: String!
}

type Pci implements Node {
  id: PrefixedID!
  type: String
  typeid: String
  vendorname: String
  vendorid: String
  productname: String
  productid: String
  blacklisted: String
  class: String
}

type Usb implements Node {
  id: PrefixedID!
  name: String
}

type Devices implements Node {
  id: PrefixedID!
  gpu: [Gpu!]!
  pci: [Pci!]!
  usb: [Usb!]!
}

type Case implements Node {
  id: PrefixedID!
  icon: String
  url: String
  error: String
  base64: String
}

type Display implements Node {
  id: PrefixedID!
  case: Case
  date: String
  number: String
  scale: Boolean
  tabs: Boolean
  users: String
  resize: Boolean
  wwn: Boolean
  total: Boolean
  usage: Boolean
  banner: String
  dashapps: String
  theme: ThemeName
  text: Boolean
  unit: Temperature
  warning: Int
  critical: Int
  hot: Int
  max: Int
  locale: String
}

"""Temperature unit (Celsius or Fahrenheit)"""
enum Temperature {
  C
  F
}

type MemoryLayout implements Node {
  id: PrefixedID!
  size: Int!
  bank: String
  type: String
  clockSpeed: Int
  formFactor: String
  manufacturer: String
  partNum: String
  serialNum: String
  voltageConfigured: Int
  voltageMin: Int
  voltageMax: Int
}

type InfoMemory implements Node {
  id: PrefixedID!
  max: Int!
  total: Int!
  free: Int!
  used: Int!
  active: Int!
  available: Int!
  buffcache: Int!
  swaptotal: Int!
  swapused: Int!
  swapfree: Int!
  layout: [MemoryLayout!]!
}

type Os implements Node {
  id: PrefixedID!
  platform: String
  distro: String
  release: String
  codename: String
  kernel: String
  arch: String
  hostname: String
  codepage: String
  logofile: String
  serial: String
  build: String
  uptime: String
}

type System implements Node {
  id: PrefixedID!
  manufacturer: String
  model: String
  version: String
  serial: String
  uuid: String
  sku: String
}

type Versions implements Node {
  id: PrefixedID!
  kernel: String
  openssl: String
  systemOpenssl: String
  systemOpensslLib: String
  node: String
  v8: String
  npm: String
  yarn: String
  pm2: String
  gulp: String
  grunt: String
  git: String
  tsc: String
  mysql: String
  redis: String
  mongodb: String
  apache: String
  nginx: String
  php: String
  docker: String
  postfix: String
  postgresql: String
  perl: String
  python: String
  gcc: String
  unraid: String
}

type Info implements Node {
  id: PrefixedID!

  """Count of docker containers"""
  apps: InfoApps!
  baseboard: Baseboard!
  cpu: InfoCpu!
  devices: Devices!
  display: Display!

  """Machine ID"""
  machineId: PrefixedID
  memory: InfoMemory!
  os: Os!
  system: System!
  time: DateTime!
  versions: Versions!
}

type ContainerPort {
  ip: String
  privatePort: Port
  publicPort: Port
  type: ContainerPortType!
}

"""
A field whose value is a valid TCP port within the range of 0 to 65535: https://en.wikipedia.org/wiki/Transmission_Control_Protocol#TCP_ports
"""
scalar Port

enum ContainerPortType {
  TCP
  UDP
}

type ContainerHostConfig {
  networkMode: String!
}

type DockerContainer implements Node {
  id: PrefixedID!
  names: [String!]!
  image: String!
  imageId: String!
  command: String!
  created: Int!
  ports: [ContainerPort!]!

  """Total size of all the files in the container"""
  sizeRootFs: Int
  labels: JSONObject
  state: ContainerState!
  status: String!
  hostConfig: ContainerHostConfig
  networkSettings: JSONObject
  mounts: [JSONObject!]
  autoStart: Boolean!
}

"""
The `JSONObject` scalar type represents JSON objects as specified by [ECMA-404](http://www.ecma-international.org/publications/files/ECMA-ST/ECMA-404.pdf).
"""
scalar JSONObject

enum ContainerState {
  RUNNING
  EXITED
}

type DockerNetwork implements Node {
  id: PrefixedID!
  name: String!
  created: String!
  scope: String!
  driver: String!
  enableIPv6: Boolean!
  ipam: JSONObject!
  internal: Boolean!
  attachable: Boolean!
  ingress: Boolean!
  configFrom: JSONObject!
  configOnly: Boolean!
  containers: JSONObject!
  options: JSONObject!
  labels: JSONObject!
}

type Docker implements Node {
  id: PrefixedID!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **DOCKER**
  - Possession: **ANY**
  """
  containers(skipCache: Boolean! = false): [DockerContainer!]!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **DOCKER**
  - Possession: **ANY**
  """
  networks(skipCache: Boolean! = false): [DockerNetwork!]!
}

type Flash implements Node {
  id: PrefixedID!
  guid: String!
  vendor: String!
  product: String!
}

type LogFile {
  """Name of the log file"""
  name: String!

  """Full path to the log file"""
  path: String!

  """Size of the log file in bytes"""
  size: Int!

  """Last modified timestamp"""
  modifiedAt: DateTime!
}

type LogFileContent {
  """Path to the log file"""
  path: String!

  """Content of the log file"""
  content: String!

  """Total number of lines in the file"""
  totalLines: Int!

  """Starting line number of the content (1-indexed)"""
  startLine: Int
}

type NotificationCounts {
  info: Int!
  warning: Int!
  alert: Int!
  total: Int!
}

type NotificationOverview {
  unread: NotificationCounts!
  archive: NotificationCounts!
}

type Notification implements Node {
  id: PrefixedID!

  """Also known as 'event'"""
  title: String!
  subject: String!
  description: String!
  importance: NotificationImportance!
  link: String
  type: NotificationType!

  """ISO Timestamp for when the notification occurred"""
  timestamp: String
  formattedTimestamp: String
}

enum NotificationImportance {
  ALERT
  INFO
  WARNING
}

enum NotificationType {
  UNREAD
  ARCHIVE
}

type Notifications implements Node {
  id: PrefixedID!

  """A cached overview of the notifications in the system & their severity."""
  overview: NotificationOverview!
  list(filter: NotificationFilter!): [Notification!]!
}

input NotificationFilter {
  importance: NotificationImportance
  type: NotificationType!
  offset: Int!
  limit: Int!
}

type Owner {
  username: String!
  url: String!
  avatar: String!
}

type VmDomain implements Node {
  """The unique identifier for the vm (uuid)"""
  id: PrefixedID!

  """A friendly name for the vm"""
  name: String

  """Current domain vm state"""
  state: VmState!

  """The UUID of the vm"""
  uuid: String @deprecated(reason: "Use id instead")
}

"""The state of a virtual machine"""
enum VmState {
  NOSTATE
  RUNNING
  IDLE
  PAUSED
  SHUTDOWN
  SHUTOFF
  CRASHED
  PMSUSPENDED
}

type Vms implements Node {
  id: PrefixedID!
  domains: [VmDomain!]
  domain: [VmDomain!]
}

type Uptime {
  timestamp: String
}

type Service implements Node {
  id: PrefixedID!
  name: String
  online: Boolean
  uptime: Uptime
  version: String
}

type UserAccount implements Node {
  id: PrefixedID!

  """The name of the user"""
  name: String!

  """A description of the user"""
  description: String!

  """The roles of the user"""
  roles: [Role!]!

  """The permissions of the user"""
  permissions: [Permission!]
}

"""
### Description:

ID scalar type that prefixes the underlying ID with the server identifier on output and strips it on input.

We use this scalar type to ensure that the ID is unique across all servers, allowing the same underlying resource ID to be used across different server instances.

#### Input Behavior:

When providing an ID as input (e.g., in arguments or input objects), the server identifier prefix ('<serverId>:') is optional.

- If the prefix is present (e.g., '123:456'), it will be automatically stripped, and only the underlying ID ('456') will be used internally.
- If the prefix is absent (e.g., '456'), the ID will be used as-is.

This makes it flexible for clients, as they don't strictly need to know or provide the server ID.

#### Output Behavior:

When an ID is returned in the response (output), it will *always* be prefixed with the current server's unique identifier (e.g., '123:456').

#### Example:

Note: The server identifier is '123' in this example.

##### Input (Prefix Optional):
```graphql
# Both of these are valid inputs resolving to internal ID '456'
{
  someQuery(id: "123:456") { ... }
  anotherQuery(id: "456") { ... }
}
```

##### Output (Prefix Always Added):
```graphql
# Assuming internal ID is '456'
{
  "data": {
    "someResource": {
      "id": "123:456" 
    }
  }
}
```
"""
scalar PrefixedID

type Query {
  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **API_KEY**
  - Possession: **ANY**
  """
  apiKeys: [ApiKey!]!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **API_KEY**
  - Possession: **ANY**
  """
  apiKey(id: PrefixedID!): ApiKey

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **CLOUD**
  - Possession: **ANY**
  """
  cloud: Cloud!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **CONFIG**
  - Possession: **ANY**
  """
  config: Config!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **DISPLAY**
  - Possession: **ANY**
  """
  display: Display!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **FLASH**
  - Possession: **ANY**
  """
  flash: Flash!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **INFO**
  - Possession: **ANY**
  """
  info: Info!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **LOGS**
  - Possession: **ANY**
  """
  logFiles: [LogFile!]!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **LOGS**
  - Possession: **ANY**
  """
  logFile(path: String!, lines: Int, startLine: Int): LogFileContent!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **ME**
  - Possession: **ANY**
  """
  me: UserAccount!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **NETWORK**
  - Possession: **ANY**
  """
  network: Network!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **NOTIFICATIONS**
  - Possession: **ANY**
  
  #### Description:
  
  Get all notifications
  """
  notifications: Notifications!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **ONLINE**
  - Possession: **ANY**
  """
  online: Boolean!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **OWNER**
  - Possession: **ANY**
  """
  owner: Owner!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **REGISTRATION**
  - Possession: **ANY**
  """
  registration: Registration

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **SERVERS**
  - Possession: **ANY**
  """
  server: Server

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **SERVERS**
  - Possession: **ANY**
  """
  servers: [Server!]!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **SERVICES**
  - Possession: **ANY**
  """
  services: [Service!]!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **SHARE**
  - Possession: **ANY**
  """
  shares: [Share!]!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **VARS**
  - Possession: **ANY**
  """
  vars: Vars!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **VMS**
  - Possession: **ANY**
  
  #### Description:
  
  Get information about all VMs on the system
  """
  vms: Vms!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **ARRAY**
  - Possession: **ANY**
  """
  parityHistory: [ParityCheck!]!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **ARRAY**
  - Possession: **ANY**
  """
  array: UnraidArray!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **CONNECT**
  - Possession: **ANY**
  """
  connect: Connect!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **CONNECT**
  - Possession: **ANY**
  """
  remoteAccess: RemoteAccess!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **CONNECT**
  - Possession: **ANY**
  """
  extraAllowedOrigins: [String!]!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **CUSTOMIZATIONS**
  - Possession: **ANY**
  """
  customization: Customization
  publicPartnerInfo: PublicPartnerInfo
  publicTheme: Theme!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **DOCKER**
  - Possession: **ANY**
  """
  docker: Docker!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **DISK**
  - Possession: **ANY**
  """
  disks: [Disk!]!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **DISK**
  - Possession: **ANY**
  """
  disk(id: PrefixedID!): Disk!
  health: String!
  getDemo: String!
}

type Mutation {
  """
  #### Required Permissions:
  
  - Action: **CREATE**
  - Resource: **API_KEY**
  - Possession: **ANY**
  """
  createApiKey(input: CreateApiKeyInput!): ApiKeyWithSecret!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **API_KEY**
  - Possession: **ANY**
  """
  addRoleForApiKey(input: AddRoleForApiKeyInput!): Boolean!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **API_KEY**
  - Possession: **ANY**
  """
  removeRoleFromApiKey(input: RemoveRoleFromApiKeyInput!): Boolean!

  """Creates a new notification record"""
  createNotification(input: NotificationData!): Notification!
  deleteNotification(id: PrefixedID!, type: NotificationType!): NotificationOverview!

  """Deletes all archived notifications on server."""
  deleteArchivedNotifications: NotificationOverview!

  """Marks a notification as archived."""
  archiveNotification(id: PrefixedID!): Notification!
  archiveNotifications(ids: [PrefixedID!]!): NotificationOverview!
  archiveAll(importance: NotificationImportance): NotificationOverview!

  """Marks a notification as unread."""
  unreadNotification(id: PrefixedID!): Notification!
  unarchiveNotifications(ids: [PrefixedID!]!): NotificationOverview!
  unarchiveAll(importance: NotificationImportance): NotificationOverview!

  """Reads each notification to recompute & update the overview."""
  recalculateOverview: NotificationOverview!
  array: ArrayMutations!
  docker: DockerMutations!
  vm: VmMutations!
  parityCheck: ParityCheckMutations!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **CONFIG**
  - Possession: **ANY**
  """
  updateApiSettings(input: ApiSettingsInput!): ConnectSettingsValues!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **CONNECT**
  - Possession: **ANY**
  """
  connectSignIn(input: ConnectSignInInput!): Boolean!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **CONNECT**
  - Possession: **ANY**
  """
  connectSignOut: Boolean!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **CONNECT**
  - Possession: **ANY**
  """
  setupRemoteAccess(input: SetupRemoteAccessInput!): Boolean!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **CONFIG**
  - Possession: **ANY**
  """
  setAdditionalAllowedOrigins(input: AllowedOriginInput!): [String!]!

  """
  #### Required Permissions:
  
  - Action: **UPDATE**
  - Resource: **CONNECT__REMOTE_ACCESS**
  - Possession: **ANY**
  """
  enableDynamicRemoteAccess(input: EnableDynamicRemoteAccessInput!): Boolean!
  setDemo: String!
}

input CreateApiKeyInput {
  name: String!
  description: String
  roles: [Role!]
  permissions: [AddPermissionInput!]

  """
  This will replace the existing key if one already exists with the same name, otherwise returns the existing key
  """
  overwrite: Boolean
}

input AddPermissionInput {
  resource: Resource!
  actions: [String!]!
}

input AddRoleForApiKeyInput {
  apiKeyId: PrefixedID!
  role: Role!
}

input RemoveRoleFromApiKeyInput {
  apiKeyId: PrefixedID!
  role: Role!
}

input NotificationData {
  title: String!
  subject: String!
  description: String!
  importance: NotificationImportance!
  link: String
}

input ApiSettingsInput {
  """
  If true, the GraphQL sandbox will be enabled and available at /graphql. If false, the GraphQL sandbox will be disabled and only the production API will be available.
  """
  sandbox: Boolean

  """A list of origins allowed to interact with the API"""
  extraOrigins: [String!]

  """The type of WAN access to use for Remote Access"""
  accessType: WAN_ACCESS_TYPE

  """The type of port forwarding to use for Remote Access"""
  forwardType: WAN_FORWARD_TYPE

  """
  The port to use for Remote Access. Not required for UPNP forwardType. Required for STATIC forwardType. Ignored if accessType is DISABLED or forwardType is UPNP.
  """
  port: Int

  """A list of Unique Unraid Account ID's"""
  ssoUserIds: [String!]
}

input ConnectSignInInput {
  """The API key for authentication"""
  apiKey: String!

  """The ID token for authentication"""
  idToken: String

  """User information for the sign-in"""
  userInfo: ConnectUserInfoInput

  """The access token for authentication"""
  accessToken: String

  """The refresh token for authentication"""
  refreshToken: String
}

input ConnectUserInfoInput {
  """The preferred username of the user"""
  preferred_username: String!

  """The email address of the user"""
  email: String!

  """The avatar URL of the user"""
  avatar: String
}

input SetupRemoteAccessInput {
  """The type of WAN access to use for Remote Access"""
  accessType: WAN_ACCESS_TYPE!

  """The type of port forwarding to use for Remote Access"""
  forwardType: WAN_FORWARD_TYPE

  """
  The port to use for Remote Access. Not required for UPNP forwardType. Required for STATIC forwardType. Ignored if accessType is DISABLED or forwardType is UPNP.
  """
  port: Int
}

input AllowedOriginInput {
  """A list of origins allowed to interact with the API"""
  origins: [String!]!
}

input EnableDynamicRemoteAccessInput {
  """The AccessURL Input for dynamic remote access"""
  url: AccessUrlInput!

  """Whether to enable or disable dynamic remote access"""
  enabled: Boolean!
}

input AccessUrlInput {
  type: URL_TYPE!
  name: String
  ipv4: URL
  ipv6: URL
}

type Subscription {
  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **DISPLAY**
  - Possession: **ANY**
  """
  displaySubscription: Display!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **INFO**
  - Possession: **ANY**
  """
  infoSubscription: Info!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **LOGS**
  - Possession: **ANY**
  """
  logFile(path: String!): LogFileContent!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **NOTIFICATIONS**
  - Possession: **ANY**
  """
  notificationAdded: Notification!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **NOTIFICATIONS**
  - Possession: **ANY**
  """
  notificationsOverview: NotificationOverview!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **OWNER**
  - Possession: **ANY**
  """
  ownerSubscription: Owner!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **REGISTRATION**
  - Possession: **ANY**
  """
  registrationSubscription: Registration!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **SERVERS**
  - Possession: **ANY**
  """
  serversSubscription: Server!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **ARRAY**
  - Possession: **ANY**
  """
  parityHistorySubscription: ParityCheck!

  """
  #### Required Permissions:
  
  - Action: **READ**
  - Resource: **ARRAY**
  - Possession: **ANY**
  """
  arraySubscription: UnraidArray!
}

"""Available authentication action verbs"""
enum AuthActionVerb {
  CREATE
  UPDATE
  DELETE
  READ
}

"""Available authentication possession types"""
enum AuthPossession {
  ANY
  OWN
  OWN_ANY
}
