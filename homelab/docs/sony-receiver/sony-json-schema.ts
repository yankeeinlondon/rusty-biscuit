/**
 * Type definitions generated from your JSON “results” list.
 *
 * Key decoding rules from the source:
 * - Embedded JSON strings like "{\"uri\":\"string\"}" => object schemas.
 * - Trailing "*" => array (e.g., "string*" => string[]; "{...}*" => Array<{...}>).
 * - "Foo[]" in strings is already an array type.
 * - int/double => number, bool/Boolean => boolean.
 *
 * Important TS modeling note:
 * Some method names appear multiple times across versions with *different* signatures
 * (e.g. getCurrentExternalTerminalsStatus() in v1.0 vs getCurrentExternalTerminalsStatus({uri}) in v1.2).
 * When we model “API vX extends vY”, we use *overloads* to keep the derived interface assignable to the base.
 */

/* ---------------------------------- */
/* Primitive aliases (optional)       */
/* ---------------------------------- */

export type Int = number;
export type Double = number;

/* ---------------------------------- */
/* Placeholder referenced model types */
/* (not defined in your snippet)      */
/* ---------------------------------- */

export type FunctionInfo = unknown;
export type SupportedFunctionInfo = unknown;

export type GeneralSettingsCandidate = unknown;
export type GeneralSettings = unknown;

export type PlaybackModeSettingsCandidate = unknown;
export type PlaybackModeSettings = unknown;

export type StateInfo = unknown;
export type DabInfo = unknown;
export type AudioInfo = unknown;
export type VideoInfo = unknown;
export type UpnpOperationInfo = unknown;

export type SubtitleInfo = unknown;
export type ParentalInfo = unknown;
export type ContentInfo = unknown;

/* ---------------------------------- */
/* Shared / utility result types      */
/* ---------------------------------- */

/** From results: getMethodTypes takes ["string"] and returns 4 entries. Model as a tuple. */
export type GetMethodTypesResult = [
  string,
  string[], // string*
  string[], // string*
  string
];

/* ---------------------------------- */
/* v1.0 types                         */
/* ---------------------------------- */

export interface GetAvailablePlaybackFunctionParams_v1_0 {
  output: string;
}
export interface GetAvailablePlaybackFunctionItem_v1_0 {
  functions: FunctionInfo[];
  output: string;
  uri: string;
}
export type GetAvailablePlaybackFunctionResult_v1_0 =
  GetAvailablePlaybackFunctionItem_v1_0[];

export interface GetBluetoothSettingsParams_v1_0 {
  target: string;
}
export interface GetBluetoothSettingsItem_v1_0 {
  target: string;
  currentValue: string;
  deviceUIInfo: string;
  title: string;
  titleTextID: string;
  type: string;
  isAvailable: boolean;
  candidate: GeneralSettingsCandidate[];
}
export type GetBluetoothSettingsResult_v1_0 = GetBluetoothSettingsItem_v1_0[];

/** v1.0 getCurrentExternalTerminalsStatus() item */
export interface ExternalTerminalStatusItem_v1_0 {
  uri: string;
  title: string;
  connection: string;
  active: string;
  label: string;
  outputs: string[];
  meta: string;
  iconUrl: string;
}
export type GetCurrentExternalTerminalsStatusResult_v1_0 =
  ExternalTerminalStatusItem_v1_0[];

export interface GetPlaybackModeSettingsParams_v1_0 {
  target: string;
  uri: string;
}
export interface GetPlaybackModeSettingsItem_v1_0 {
  target: string;
  uri: string;
  currentValue: string;
  deviceUIInfo: string;
  title: string;
  titleTextID: string;
  type: string;
  isAvailable: boolean;
  candidate: PlaybackModeSettingsCandidate[];
}
export type GetPlaybackModeSettingsResult_v1_0 = GetPlaybackModeSettingsItem_v1_0[];

export interface SchemeItem_v1_0 {
  scheme: string;
}
export type GetSchemeListResult_v1_0 = SchemeItem_v1_0[];

export interface GetSupportedPlaybackFunctionParams_v1_0 {
  uri: string;
}
export interface GetSupportedPlaybackFunctionItem_v1_0 {
  functions: SupportedFunctionInfo[];
  uri: string;
}
export type GetSupportedPlaybackFunctionResult_v1_0 =
  GetSupportedPlaybackFunctionItem_v1_0[];

export type GetVersionsResult_v1_0 = string[];

export interface PresetBroadcastStationParams_v1_0 {
  uri: string;
  frequency: Int;
}

export interface ScanPlayingContentParams_v1_0 {
  output: string;
  direction: string;
}

export interface SeekBroadcastStationParams_v1_0 {
  direction: string;
  tuning: string;
}

export interface SetActiveTerminalParams_v1_0 {
  uri: string;
  active: string;
}

export interface SetBluetoothSettingsParams_v1_0 {
  settings: GeneralSettings[];
}

export interface SetPlayNextContentParams_v1_0 {
  output: string;
}

export interface SetPlayPreviousContentParams_v1_0 {
  output: string;
}

export interface StartContentBrowsingParams_v1_0 {
  uri: string;
}
export interface StartContentBrowsingResult_v1_0 {
  status: string;
  errorMessage: string;
}

/* ---------------------------------- */
/* v1.1 types                         */
/* ---------------------------------- */

export interface PausePlayingContentParams_v1_1 {
  output: string;
}

export interface SetPlaybackModeSettingsParams_v1_1 {
  settings: PlaybackModeSettings[];
}

export interface StopPlayingContentParams_v1_1 {
  output: string;
  keepLastFrame: boolean;
}

/* ---------------------------------- */
/* v1.2 types                         */
/* ---------------------------------- */

export interface GetCurrentExternalTerminalsStatusParams_v1_2 {
  uri: string;
}
export interface ExternalTerminalStatusItem_v1_2 {
  uri: string;
  title: string;
  connection: string;
  active: string;
  label: string;
  outputs: string[];
  meta: string;
  iconUrl: string;
  avOutput: string;
  iconId: string;
}
export type GetCurrentExternalTerminalsStatusResult_v1_2 =
  ExternalTerminalStatusItem_v1_2[];

export interface GetPlayingContentInfoParams_v1_2 {
  output: string;
}
export interface PlayingContentInfoItem_v1_2 {
  uri: string;
  source: string;
  sourceLabel: string;
  title: string;
  output: string;

  stateInfo: StateInfo;

  positionSec: Double;
  positionMsec: Int;
  durationSec: Double;
  durationMsec: Int;

  playSpeedStep: Int;
  repeatType: string;

  dispNum: string;
  originalDispNum: string;
  tripletStr: string;

  programNum: Int;
  programTitle: string;
  startDateTime: string;

  mediaType: string;
  playSpeed: string;

  bivl_serviceId: string;
  bivl_assetId: string;
  bivl_provider: string;

  chapterIndex: Int;
  chapterCount: Int;

  subtitleIndex: Int;

  artist: string;
  genre: string[];

  albumName: string;
  contentKind: string;

  fileNo: string;
  channelName: string;

  playlistName: string;
  podcastName: string;

  totalCount: Int;

  broadcastFreq: Int;
  broadcastFreqBand: string;

  dabInfo: DabInfo;
  audioInfo: AudioInfo[];

  parentUri: string;
  service: string;
  index: Int;

  videoInfo: VideoInfo;
  applicationName: string;
}
export type GetPlayingContentInfoResult_v1_2 = PlayingContentInfoItem_v1_2[];

export interface GetSourceListParams_v1_2 {
  scheme: string;
}
/** v1.2 getSourceList item */
export interface SourceListItem_v1_2 {
  source: string;
  title: string;
  isPlayable: boolean;
  isBrowsable: boolean;
  playAction: string;
  outputs: string[];
  meta: string;
  iconUrl: string;
  protocols: string[];
  upnpOperationInfo: UpnpOperationInfo;
}
export type GetSourceListResult_v1_2 = SourceListItem_v1_2[];

export interface SetPlayContentParams_v1_2 {
  uri: string;
  positionSec: Double;
  positionMsec: Int;
  resume: boolean;
  requester: string;
  repeatType: string;
  keepLastFrame: boolean;
  output: string;
}

/* ---------------------------------- */
/* v1.3 types                         */
/* ---------------------------------- */

export interface GetContentCountParams_v1_3 {
  uri: string;
  type: string[];
  target: string;
  view: string;
}
export interface GetContentCountResult_v1_3 {
  count: Int;
  capability: Int;
}

/** v1.3 getSourceList item adds iconId */
export interface SourceListItem_v1_3 extends SourceListItem_v1_2 {
  iconId: string;
}
export type GetSourceListResult_v1_3 = SourceListItem_v1_3[];

/* ---------------------------------- */
/* v1.4 types                         */
/* ---------------------------------- */

export interface GetContentListParams_v1_4 {
  uri: string;
  stIdx: Int;
  cnt: Int;
  type: string[];
  target: string;
  view: string;
  sort: string;
}

export interface ContentListItem_v1_4 {
  uri: string;
  title: string;
  index: Int;

  dispNum: string;
  originalDispNum: string;
  tripletStr: string;

  programNum: Int;
  programMediaType: string;

  directRemoteNum: Int;

  epgVisibility: string;
  channelSurfingVisibility: string;
  visibility: string;

  startDateTime: string;
  channelName: string;

  fileSizeByte: Int;

  isProtected: string;
  isAlreadyPlayed: string;

  productID: string;
  contentType: string;
  storageUri: string;

  chapterCount: Int;
  durationMsec: Int;

  audioInfo: AudioInfo[];
  subtitleInfo: SubtitleInfo[];
  parentalInfo: ParentalInfo[];

  sizeMB: Int;
  createdTime: string;

  userContentFlag: boolean;

  content: ContentInfo;

  folderNo: string;
  fileNo: string;

  artist: string;
  genre: string[];
  albumName: string;
  contentKind: string;

  isPlayable: string;
  isBrowsable: string;

  remotePlayType: string[];

  playlistName: string;
  podcastName: string;

  broadcastFreq: Int;
  broadcastFreqBand: string;

  parentUri: string;

  videoInfo: VideoInfo;
}
export type GetContentListResult_v1_4 = ContentListItem_v1_4[];

/* ---------------------------------- */
/* API interfaces by version          */
/* ---------------------------------- */

/**
 * API v1.0
 *
 * Matches your results list at version "1.0".
 */
export interface Api_v1_0 {
  // ---- Queries / getters ----
  getAvailablePlaybackFunction(
    params: GetAvailablePlaybackFunctionParams_v1_0
  ): GetAvailablePlaybackFunctionResult_v1_0;

  getBluetoothSettings(
    params: GetBluetoothSettingsParams_v1_0
  ): GetBluetoothSettingsResult_v1_0;

  /** v1.0 form (no params) */
  getCurrentExternalTerminalsStatus(): GetCurrentExternalTerminalsStatusResult_v1_0;

  /** Returns: [string, string*, string*, string] */
  getMethodTypes(arg0: string): GetMethodTypesResult;

  getPlaybackModeSettings(
    params: GetPlaybackModeSettingsParams_v1_0
  ): GetPlaybackModeSettingsResult_v1_0;

  getSchemeList(): GetSchemeListResult_v1_0;

  getSupportedPlaybackFunction(
    params: GetSupportedPlaybackFunctionParams_v1_0
  ): GetSupportedPlaybackFunctionResult_v1_0;

  getVersions(): GetVersionsResult_v1_0;

  // ---- Commands / setters ----
  presetBroadcastStation(params: PresetBroadcastStationParams_v1_0): void;

  scanPlayingContent(params: ScanPlayingContentParams_v1_0): void;

  seekBroadcastStation(params: SeekBroadcastStationParams_v1_0): void;

  setActiveTerminal(params: SetActiveTerminalParams_v1_0): void;

  setBluetoothSettings(params: SetBluetoothSettingsParams_v1_0): void;

  setPlayNextContent(params: SetPlayNextContentParams_v1_0): void;

  setPlayPreviousContent(params: SetPlayPreviousContentParams_v1_0): void;

  startContentBrowsing(
    params: StartContentBrowsingParams_v1_0
  ): StartContentBrowsingResult_v1_0;
}

/**
 * API v1.1
 *
 * Adds pausePlayingContent, setPlaybackModeSettings, stopPlayingContent.
 */
export interface Api_v1_1 extends Api_v1_0 {
  pausePlayingContent(params: PausePlayingContentParams_v1_1): void;

  setPlaybackModeSettings(params: SetPlaybackModeSettingsParams_v1_1): void;

  stopPlayingContent(params: StopPlayingContentParams_v1_1): void;
}

/**
 * API v1.2
 *
 * Adds:
 * - getCurrentExternalTerminalsStatus({uri}) (overload) with richer return item
 * - getPlayingContentInfo
 * - getSourceList (v1.2 shape)
 * - setPlayContent
 *
 * Uses overload to remain assignable to v1.1 (which has the no-arg method).
 */
export interface Api_v1_2 extends Api_v1_1 {
  // ---- Overrides via overloads ----

  /** v1.0 legacy form */
  getCurrentExternalTerminalsStatus(): GetCurrentExternalTerminalsStatusResult_v1_0;

  /** v1.2 uri-filtered form */
  getCurrentExternalTerminalsStatus(
    params: GetCurrentExternalTerminalsStatusParams_v1_2
  ): GetCurrentExternalTerminalsStatusResult_v1_2;

  // ---- New in v1.2 ----
  getPlayingContentInfo(
    params: GetPlayingContentInfoParams_v1_2
  ): GetPlayingContentInfoResult_v1_2;

  getSourceList(params: GetSourceListParams_v1_2): GetSourceListResult_v1_2;

  setPlayContent(params: SetPlayContentParams_v1_2): void;
}

/**
 * API v1.3
 *
 * Adds:
 * - getContentCount
 * - getSourceList return item adds iconId (override via overload/widening)
 *
 * Because the v1.3 getSourceList result is a *wider item type* (superset),
 * it is safe to “override” by returning the v1.3 result type.
 */
export interface Api_v1_3 extends Api_v1_2 {
  getContentCount(params: GetContentCountParams_v1_3): GetContentCountResult_v1_3;

  // Override: keep signature compatible, widen return type to v1.3
  getSourceList(params: GetSourceListParams_v1_2): GetSourceListResult_v1_3;
}

/**
 * API v1.4
 *
 * Adds getContentList.
 */
export interface Api_v1_4 extends Api_v1_3 {
  getContentList(params: GetContentListParams_v1_4): GetContentListResult_v1_4;
}
