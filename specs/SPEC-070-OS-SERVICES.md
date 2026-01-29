# SPEC-070: OS and Services Model

## Status
Draft v0.2

## Purpose
Define the model for OS services, syscalls, and IPC used by recompiled titles.

## Goals
- Provide a stable service layer for essential game functionality.
- Support gradual expansion of services as needed.

## Non-Goals
- Complete OS emulation.
- Full fidelity implementation of all system services.

## Background
- Switch titles interact with system services via IPC; service handles are obtained through the service manager `sm:` and `sm:m`.
- Process service access is mediated by `sm:m` RegisterProcess, which uses ACID/ACI0 lists from NPDM metadata.

## Service Categories
- Process and thread management (pm, sm, svc interface).
- File and storage access (fs, loader, content management).
- Input and controller services (hid, irs families).
- Audio services (audout, audren).
- Applet and UI services (applet, ns families).
- Networking and online services (initially stubbed or disabled).

## Requirements
- Service discovery and versioning.
- Service access control checks against NPDM ACID/ACI0 lists.
- Stub behavior for unsupported services.
- Controlled access to host filesystem and devices.
- Deterministic logging for service calls and IPC replies.

## IPC Protocols and Message Formats
- HIPC provides the transport. CMIF is used by most services; TIPC is used by the Service Manager and pgl.
- CMIF raw data embeds CmifInHeader/CmifOutHeader (signatures `SFCI`/`SFCO`), MethodId, Result, and (14.0.0+) InterfaceId.
- CMIF domains multiplex multiple objects under one handle using CmifDomainMessage headers.
- TIPC does not use domains; the request ID is stored in the low 16 bits of Header0Tag, and raw data is the payload.
- Service Manager switched to TIPC in 12.0.0+, with a CMIF shim retained for older titles.

## CMIF/TIPC Byte-Level Header Diagrams
These diagrams show the common on-wire layouts for implementers. Sizes are in bytes and fields are little-endian unless noted. Bitfield definitions follow the HIPC format.

### HIPC Message (Common Layout)
```
0x00  u32  Header0 (Header0Tag)
0x04  u32  Header1 (Header1Tag)
0x08  u32  SpecialHeader (optional, if Header1.SpecialCount=1)
0x0C  u64  ProcessId (optional, if SpecialHeader.Pid=1)
0x14  u32  CopyHandle[CopyHandleCount]
0x??  u32  MoveHandle[MoveHandleCount]
0x??  PointerData[PointerCount]
0x??  MapData[SendCount]
0x??  MapData[ReceiveCount]
0x??  MapData[ExchangeCount]
0x??  RawData[RawCount * 4]
0x??  ReceiveListData[ReceiveListCount]
```

### Header0Tag Bitfields
```
bits  0-15  Tag (MessageType)
bits 16-19  PointerCount
bits 20-23  SendCount
bits 24-27  ReceiveCount
bits 28-31  ExchangeCount
```

### Header1Tag Bitfields
```
bits  0-9   RawCount (u32 words)
bits 10-13  ReceiveListCount
bits 14-19  Reserved
bits 20-30  ReceiveListOffset (u32 words from start of message)
bit  31     SpecialCount
```

### SpecialHeader (SpecialTag) Bitfields
```
bit   0     Pid
bits  1-4   CopyHandleCount
bits  5-8   MoveHandleCount
bits  9-31  Reserved
```

### PointerData (Pointer Buffer Descriptor)
```
Data0 (Pointer0Tag):
  bits  0-3   PointerIndex
  bits  4-5   Reserved
  bits  6-8   PointerAddress36 (bits 36..38)
  bits  9-11  Reserved
  bits 12-15  PointerAddress32 (bits 32..35)
  bits 16-31  PointerSize
Data1 (Pointer1Tag):
  bits  0-31  PointerAddress0 (bits 0..31)
```

### MapData (Send/Receive/Exchange Buffer Descriptor)
```
Data0 (Map0Tag):
  bits  0-31  MapSizeLow (bits 0..31)
Data1 (Map1Tag):
  bits  0-31  MapAddress0 (bits 0..31)
Data2 (Map2Tag):
  bits   0-1  MapTransferAttribute
  bits   2-4  MapAddress36 (bits 36..38)
  bits   5-23 Reserved
  bits  24-27 MapSizeHi (bits 32..35)
  bits  28-31 MapAddress32 (bits 32..35)
```

### ReceiveListData (Receive List Descriptor)
```
Data0 (ReceiveList0Tag):
  bits  0-31  ReceiveListAddressLow (bits 0..31)
Data1 (ReceiveList1Tag):
  bits  0-6   ReceiveListAddressHi (bits 32..38)
  bits  7-15  Reserved
  bits 16-31  ReceiveListSize
```

### Handle Descriptor Layout
```
CopyHandle[CopyHandleCount]  u32 handles (copy semantics)
MoveHandle[MoveHandleCount]  u32 handles (move semantics)
```

### CMIF Raw Data (Non-Domain Request)
```
0x00  u32  Magic = 'SFCI'
0x04  u16  Version
0x06  u16  Reserved
0x08  u32  CommandId
0x0C  u32  Token (0 unless object is bound)
0x10  ...  Parameters
```

### CMIF Raw Data (Non-Domain Response)
```
0x00  u32  Magic = 'SFCO'
0x04  u16  Version
0x06  u16  Reserved
0x08  u32  Result
0x0C  u32  Token (0 unless object is bound)
0x10  ...  Return values
```

### CMIF Domain Message Header (Request)
```
0x00  u8   Type (1 = request)
0x01  u8   NumObjIds
0x02  u16  Reserved
0x04  u32  ObjectId
0x08  u32  Padding
0x0C  ...  Embedded CMIF header + parameters
```

### CMIF Domain Message Header (Response)
```
0x00  u8   Type (2 = response)
0x01  u8   NumObjIds
0x02  u16  Reserved
0x04  u32  ObjectId
0x08  u32  Padding
0x0C  ...  Embedded CMIF header + return values
```

### TIPC Raw Data (Service Manager, pgl)
```
0x00  u32  Header0Tag (lower 16 bits = RequestId)
0x04  u32  Header1
0x08  u32  Header2
0x0C  u32  Header3
0x10  ...  Parameters / returns
```

## Service Surface (Initial Targets)
- Service manager: `sm:` and `sm:m` with minimal client registration and handle lookup.
- Input: `hid` service family (hid, hid:sys, irs).
- Audio: `audout` and `audren` families (user-facing variants).
- Applet manager: `appletOE` for basic applet interactions.
- Account and NS services as stubs for title bootstrapping (acc:u0, ns:am).

## Command IDs (Initial Baseline)
The following are the minimum command IDs required for early bring-up. Version-gating is required where noted.

### sm: (nn::sm::detail::IUserInterface)
- 0 RegisterClient
- 1 GetServiceHandle
- 2 RegisterService (TIPC-only, 12.0.0+)
- 3 UnregisterService (TIPC-only, 12.0.0+)
- 4 DetachClient (11.0.0-11.0.1)

### sm:m (nn::sm::detail::IManagerInterface)
- 0 RegisterProcess
- 1 UnregisterProcess

### hid (nn::hid::IHidServer)
- 0 CreateAppletResource
- 11 ActivateTouchScreen
- 31 ActivateKeyboard
- 51 ActivateXpad
- 55 GetXpadIds
- 60 ActivateSixAxisSensor
- 62 GetSixAxisSensorLifoHandle

### audout:u (nn::audio::detail::IAudioOutManager)
- 0 ListAudioOuts
- 1 OpenAudioOut
- 2 ListAudioOutsAuto (3.0.0+)
- 3 OpenAudioOutAuto (3.0.0+)

### audren:u (nn::audio::detail::IAudioRendererManager)
- 0 OpenAudioRenderer
- 1 GetWorkBufferSize
- 2 GetAudioDeviceService
- 3 OpenAudioRendererForManualExecution (3.0.0+)
- 4 GetAudioDeviceServiceWithRevisionInfo (4.0.0+)

### appletOE (Applet Manager services)
### appletOE (nn::am::service::IApplicationProxyService)
- 0 OpenApplicationProxy -> IApplicationProxy

### acc:u0 (nn::account::IAccountServiceForApplication)
- 0 GetUserCount
- 1 GetUserExistence
- 2 ListAllUsers
- 3 ListOpenUsers
- 4 GetLastOpenedUser
- 5 GetProfile
- 6 GetProfileDigest (3.0.0+)
- 50 IsUserRegistrationRequestPermitted
- 51 TrySelectUserWithoutInteractionDeprecated (1.0.0-18.1.0)
- 52 TrySelectUserWithoutInteraction (19.0.0+)
- 100 InitializeApplicationInfoV0
- 101 GetBaasAccountManagerForApplication

### ns:am (nn::ns::detail::IApplicationManagerInterface)
- 0 ListApplicationRecord
- 19 LaunchApplication
- 21 LaunchLibraryApplet
- 22 GetApplicationContentPath
- 23 LaunchSystemApplet
- 24 TerminateApplication
- 25 LaunchOverlayApplet
- 30 GetApplicationLogoData
- 400 GetApplicationControlData


## Deliverables
- A service dispatch table.
- A stub framework with structured errors.
- A service access control validator based on NPDM metadata.

## Open Questions
- Which service calls are required by the first target titles?
- How should online services be handled for preservation builds?
- Which sysmodules must be modeled vs stubbed for early milestones?

## Acceptance Criteria
- A minimal title reaches a main loop with only stubbed services.
- Service calls are logged with deterministic ordering.

## References
- https://switchbrew.org/wiki/HIPC
- https://switchbrew.org/wiki/Services_API
- https://switchbrew.org/wiki/12.0.0
- https://switchbrew.org/wiki/HID_services
- https://switchbrew.org/wiki/Audio_services
- https://switchbrew.org/wiki/Account_services
- https://switchbrew.org/wiki/NS_services
- https://reswitched.github.io/SwIPC/ifaces.html
- https://switchbrew.org/wiki/NPDM
- https://www.switchbrew.org/wiki/Title_list
