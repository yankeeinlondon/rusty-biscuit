---
kind: reference
created: 2026-10-13
---

# Endpoint inventory (Phase 1 baseline)

Generated on 2026-10-13 from the generated `schematic/schema/src` (working tree at `7aae49e68` plus the uncommitted runtime-envelope repair), by parsing each `EndpointSpec` impl, its `{Request}::new` signature, and its `with_*` setters.
Method names use the generator's `to_snake_case`. **Call type** is the request type's name with its trailing `Request` replaced by `Call`, which keeps the two EMQX APIs and the two Ollama APIs apart.

- **APIs:** 18. **Endpoints:** 547. **Optional setters:** 410.
- **D2 / ruling 7 collisions:** none. The expected rename list for the shipped definitions is empty.
- `new()`: — means the request has no `new()` and is built with `Default`, so the endpoint method takes no arguments.

## Anthropic (`anthropic`, 4)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| CountTokens | `count_tokens` | POST | `body` |  | `CountTokensCall` |
| CreateMessage | `create_message` | POST | `body` |  | `CreateMessageCall` |
| ListModels | `list_models` | GET | — |  | `ListModelsCall` |
| RetrieveModel | `retrieve_model` | GET | `model_id` |  | `RetrieveModelCall` |

## ArtificialAnalysisCritPt (`artificial_analysis`, 1)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| EvaluateCritPt | `evaluate_crit_pt` | POST | `body` |  | `EvaluateCritPtCall` |

## ArtificialAnalysisData (`artificial_analysis`, 6)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| ListImageEditingModels | `list_image_editing_models` | GET | — |  | `ListImageEditingModelsCall` |
| ListImageToVideoModels | `list_image_to_video_models` | GET | (none) | `include_categories` | `ListImageToVideoModelsCall` |
| ListLlmModels | `list_llm_models` | GET | — |  | `ListLlmModelsCall` |
| ListTextToImageModels | `list_text_to_image_models` | GET | (none) | `include_categories` | `ListTextToImageModelsCall` |
| ListTextToSpeechModels | `list_text_to_speech_models` | GET | — |  | `ListTextToSpeechModelsCall` |
| ListTextToVideoModels | `list_text_to_video_models` | GET | (none) | `include_categories` | `ListTextToVideoModelsCall` |

## Bitbucket (`bitbucket`, 15)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| GetBranch | `get_branch` | GET | `workspace`, `repo_slug`, `name` |  | `GetBranchCall` |
| GetDownload | `get_download` | GET | `workspace`, `repo_slug`, `filename` |  | `GetDownloadCall` |
| GetIssue | `get_issue` | GET | `workspace`, `repo_slug`, `id` |  | `GetIssueCall` |
| GetPullRequest | `get_pull_request` | GET | `workspace`, `repo_slug`, `id` |  | `GetPullRequestCall` |
| GetRepository | `get_repository` | GET | `workspace`, `repo_slug` |  | `GetRepositoryCall` |
| GetSource | `get_source` | GET | `workspace`, `repo_slug`, `commit`, `path` | `page`, `pagelen` | `GetSourceCall` |
| GetTag | `get_tag` | GET | `workspace`, `repo_slug`, `name` |  | `GetTagCall` |
| ListDownloads | `list_downloads` | GET | `workspace`, `repo_slug` | `page`, `pagelen` | `ListDownloadsCall` |
| ListIssueChanges | `list_issue_changes` | GET | `workspace`, `repo_slug`, `id` | `page`, `pagelen` | `ListIssueChangesCall` |
| ListIssueComments | `list_issue_comments` | GET | `workspace`, `repo_slug`, `id` | `page`, `pagelen` | `ListIssueCommentsCall` |
| ListIssues | `list_issues` | GET | `workspace`, `repo_slug` | `page`, `pagelen` | `ListIssuesCall` |
| ListPullRequestComments | `list_pull_request_comments` | GET | `workspace`, `repo_slug`, `id` | `page`, `pagelen` | `ListPullRequestCommentsCall` |
| ListPullRequests | `list_pull_requests` | GET | `workspace`, `repo_slug` | `page`, `pagelen`, `state` | `ListPullRequestsCall` |
| ListTags | `list_tags` | GET | `workspace`, `repo_slug` | `page`, `pagelen` | `ListTagsCall` |
| ListWorkspaceRepos | `list_workspace_repos` | GET | `workspace` | `page`, `pagelen` | `ListWorkspaceReposCall` |

## ElevenLabs (`elevenlabs`, 42)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| AddSharedVoice | `add_shared_voice` | POST | `public_user_id`, `voice_id`, `body` |  | `AddSharedVoiceCall` |
| AddVoiceSample | `add_voice_sample` | POST | `voice_id`, `body` |  | `AddVoiceSampleCall` |
| CopyResourceToWorkspace | `copy_resource_to_workspace` | POST | `resource_id`, `body` |  | `CopyResourceToWorkspaceCall` |
| CreateApiKey | `create_api_key` | POST | `service_account_user_id`, `body` |  | `CreateApiKeyCall` |
| CreatePvcVoice | `create_pvc_voice` | POST | `body` |  | `CreatePvcVoiceCall` |
| CreateSingleUseToken | `create_single_use_token` | POST | `token_type` |  | `CreateSingleUseTokenCall` |
| CreateSoundEffect | `create_sound_effect` | POST | `body` |  | `CreateSoundEffectCall` |
| CreateSpeech | `create_speech` | POST | `voice_id`, `body` |  | `CreateSpeechCall` |
| CreateSpeechWithTimestamps | `create_speech_with_timestamps` | POST | `voice_id`, `body` |  | `CreateSpeechWithTimestampsCall` |
| CreateWebhook | `create_webhook` | POST | `body` |  | `CreateWebhookCall` |
| DeleteApiKey | `delete_api_key` | DELETE | `service_account_user_id`, `api_key_id` |  | `DeleteApiKeyCall` |
| DeleteHistoryItem | `delete_history_item` | DELETE | `history_item_id` |  | `DeleteHistoryItemCall` |
| DeleteVoice | `delete_voice` | DELETE | `voice_id` |  | `DeleteVoiceCall` |
| DeleteVoiceSample | `delete_voice_sample` | DELETE | `voice_id`, `sample_id` |  | `DeleteVoiceSampleCall` |
| DeleteWebhook | `delete_webhook` | DELETE | `webhook_id` |  | `DeleteWebhookCall` |
| DownloadHistoryItems | `download_history_items` | POST | `body` |  | `DownloadHistoryItemsCall` |
| GetDefaultVoiceSettings | `get_default_voice_settings` | GET | — |  | `GetDefaultVoiceSettingsCall` |
| GetHistory | `get_history` | GET | — |  | `GetHistoryCall` |
| GetHistoryItem | `get_history_item` | GET | `history_item_id` |  | `GetHistoryItemCall` |
| GetHistoryItemAudio | `get_history_item_audio` | GET | `history_item_id` |  | `GetHistoryItemAudioCall` |
| GetResource | `get_resource` | GET | `resource_id` |  | `GetResourceCall` |
| GetUsageStats | `get_usage_stats` | GET | — |  | `GetUsageStatsCall` |
| GetUser | `get_user` | GET | — |  | `GetUserCall` |
| GetUserSubscription | `get_user_subscription` | GET | — |  | `GetUserSubscriptionCall` |
| GetVoice | `get_voice` | GET | `voice_id` |  | `GetVoiceCall` |
| GetVoiceSampleAudio | `get_voice_sample_audio` | GET | `voice_id`, `sample_id` |  | `GetVoiceSampleAudioCall` |
| GetVoiceSettings | `get_voice_settings` | GET | `voice_id` |  | `GetVoiceSettingsCall` |
| ListModels | `list_models` | GET | — |  | `ListModelsCall` |
| ListServiceAccountApiKeys | `list_service_account_api_keys` | GET | `service_account_user_id` |  | `ListServiceAccountApiKeysCall` |
| ListServiceAccounts | `list_service_accounts` | GET | — |  | `ListServiceAccountsCall` |
| ListSharedVoices | `list_shared_voices` | GET | — |  | `ListSharedVoicesCall` |
| ListVoices | `list_voices` | GET | (none) | `page_size`, `page_token` | `ListVoicesCall` |
| ListWebhooks | `list_webhooks` | GET | — |  | `ListWebhooksCall` |
| ShareResource | `share_resource` | POST | `resource_id`, `body` |  | `ShareResourceCall` |
| StreamSpeech | `stream_speech` | POST | `voice_id`, `body` |  | `StreamSpeechCall` |
| StreamSpeechWithTimestamps | `stream_speech_with_timestamps` | POST | `voice_id`, `body` |  | `StreamSpeechWithTimestampsCall` |
| TrainPvcVoice | `train_pvc_voice` | POST | `voice_id`, `body` |  | `TrainPvcVoiceCall` |
| UnshareResource | `unshare_resource` | POST | `resource_id`, `body` |  | `UnshareResourceCall` |
| UpdateApiKey | `update_api_key` | PATCH | `service_account_user_id`, `api_key_id`, `body` |  | `UpdateApiKeyCall` |
| UpdatePvcVoice | `update_pvc_voice` | POST | `voice_id`, `body` |  | `UpdatePvcVoiceCall` |
| UpdateVoiceSettings | `update_voice_settings` | POST | `voice_id`, `body` |  | `UpdateVoiceSettingsCall` |
| UpdateWebhook | `update_webhook` | PATCH | `webhook_id`, `body` |  | `UpdateWebhookCall` |

## EmqxBasic (`emqx`, 36)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| CreateAuthUser | `create_auth_user` | POST | `id`, `body` |  | `CreateAuthUserBasicCall` |
| CreateBan | `create_ban` | POST | `body` |  | `CreateBanBasicCall` |
| CreateRule | `create_rule` | POST | `body` |  | `CreateRuleBasicCall` |
| DeleteAuthUser | `delete_auth_user` | DELETE | `id`, `user_id` |  | `DeleteAuthUserBasicCall` |
| DeleteBan | `delete_ban` | DELETE | `ban_type`, `who` |  | `DeleteBanBasicCall` |
| DeleteRetained | `delete_retained` | DELETE | `topic` |  | `DeleteRetainedBasicCall` |
| DeleteRule | `delete_rule` | DELETE | `id` |  | `DeleteRuleBasicCall` |
| DisconnectClient | `disconnect_client` | DELETE | `clientid` |  | `DisconnectClientBasicCall` |
| GetAuthenticator | `get_authenticator` | GET | `id` |  | `GetAuthenticatorBasicCall` |
| GetClient | `get_client` | GET | `clientid` |  | `GetClientBasicCall` |
| GetCluster | `get_cluster` | GET | — |  | `GetClusterBasicCall` |
| GetListener | `get_listener` | GET | `id` |  | `GetListenerBasicCall` |
| GetNode | `get_node` | GET | `node` |  | `GetNodeBasicCall` |
| GetPrometheus | `get_prometheus` | GET | — |  | `GetPrometheusBasicCall` |
| GetRetained | `get_retained` | GET | `topic` |  | `GetRetainedBasicCall` |
| GetRule | `get_rule` | GET | `id` |  | `GetRuleBasicCall` |
| ListAlarms | `list_alarms` | GET | — |  | `ListAlarmsBasicCall` |
| ListAuthUsers | `list_auth_users` | GET | `id` |  | `ListAuthUsersBasicCall` |
| ListAuthenticators | `list_authenticators` | GET | — |  | `ListAuthenticatorsBasicCall` |
| ListAuthzSources | `list_authz_sources` | GET | — |  | `ListAuthzSourcesBasicCall` |
| ListBanned | `list_banned` | GET | — |  | `ListBannedBasicCall` |
| ListClients | `list_clients` | GET | — |  | `ListClientsBasicCall` |
| ListListeners | `list_listeners` | GET | — |  | `ListListenersBasicCall` |
| ListMetrics | `list_metrics` | GET | — |  | `ListMetricsBasicCall` |
| ListNodes | `list_nodes` | GET | — |  | `ListNodesBasicCall` |
| ListRetained | `list_retained` | GET | — |  | `ListRetainedBasicCall` |
| ListRules | `list_rules` | GET | — |  | `ListRulesBasicCall` |
| ListStats | `list_stats` | GET | — |  | `ListStatsBasicCall` |
| ListSubscriptions | `list_subscriptions` | GET | — |  | `ListSubscriptionsBasicCall` |
| ListTopics | `list_topics` | GET | — |  | `ListTopicsBasicCall` |
| Publish | `publish` | POST | `body` |  | `PublishBasicCall` |
| PublishBulk | `publish_bulk` | POST | `body` |  | `PublishBulkBasicCall` |
| SubscribeClient | `subscribe_client` | POST | `clientid`, `body` |  | `SubscribeClientBasicCall` |
| TestRule | `test_rule` | POST | `id`, `body` |  | `TestRuleBasicCall` |
| UnsubscribeClient | `unsubscribe_client` | POST | `clientid`, `body` |  | `UnsubscribeClientBasicCall` |
| UpdateRule | `update_rule` | PUT | `id`, `body` |  | `UpdateRuleBasicCall` |

## EmqxBearer (`emqx`, 38)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| CreateAuthUser | `create_auth_user` | POST | `id`, `body` |  | `CreateAuthUserBearerCall` |
| CreateBan | `create_ban` | POST | `body` |  | `CreateBanBearerCall` |
| CreateRule | `create_rule` | POST | `body` |  | `CreateRuleBearerCall` |
| DeleteAuthUser | `delete_auth_user` | DELETE | `id`, `user_id` |  | `DeleteAuthUserBearerCall` |
| DeleteBan | `delete_ban` | DELETE | `ban_type`, `who` |  | `DeleteBanBearerCall` |
| DeleteRetained | `delete_retained` | DELETE | `topic` |  | `DeleteRetainedBearerCall` |
| DeleteRule | `delete_rule` | DELETE | `id` |  | `DeleteRuleBearerCall` |
| DisconnectClient | `disconnect_client` | DELETE | `clientid` |  | `DisconnectClientBearerCall` |
| GetAuthenticator | `get_authenticator` | GET | `id` |  | `GetAuthenticatorBearerCall` |
| GetClient | `get_client` | GET | `clientid` |  | `GetClientBearerCall` |
| GetCluster | `get_cluster` | GET | — |  | `GetClusterBearerCall` |
| GetListener | `get_listener` | GET | `id` |  | `GetListenerBearerCall` |
| GetNode | `get_node` | GET | `node` |  | `GetNodeBearerCall` |
| GetPrometheus | `get_prometheus` | GET | — |  | `GetPrometheusBearerCall` |
| GetRetained | `get_retained` | GET | `topic` |  | `GetRetainedBearerCall` |
| GetRule | `get_rule` | GET | `id` |  | `GetRuleBearerCall` |
| ListAlarms | `list_alarms` | GET | — |  | `ListAlarmsBearerCall` |
| ListAuthUsers | `list_auth_users` | GET | `id` |  | `ListAuthUsersBearerCall` |
| ListAuthenticators | `list_authenticators` | GET | — |  | `ListAuthenticatorsBearerCall` |
| ListAuthzSources | `list_authz_sources` | GET | — |  | `ListAuthzSourcesBearerCall` |
| ListBanned | `list_banned` | GET | — |  | `ListBannedBearerCall` |
| ListClients | `list_clients` | GET | — |  | `ListClientsBearerCall` |
| ListListeners | `list_listeners` | GET | — |  | `ListListenersBearerCall` |
| ListMetrics | `list_metrics` | GET | — |  | `ListMetricsBearerCall` |
| ListNodes | `list_nodes` | GET | — |  | `ListNodesBearerCall` |
| ListRetained | `list_retained` | GET | — |  | `ListRetainedBearerCall` |
| ListRules | `list_rules` | GET | — |  | `ListRulesBearerCall` |
| ListStats | `list_stats` | GET | — |  | `ListStatsBearerCall` |
| ListSubscriptions | `list_subscriptions` | GET | — |  | `ListSubscriptionsBearerCall` |
| ListTopics | `list_topics` | GET | — |  | `ListTopicsBearerCall` |
| Login | `login` | POST | `body` |  | `LoginBearerCall` |
| Logout | `logout` | POST | — |  | `LogoutBearerCall` |
| Publish | `publish` | POST | `body` |  | `PublishBearerCall` |
| PublishBulk | `publish_bulk` | POST | `body` |  | `PublishBulkBearerCall` |
| SubscribeClient | `subscribe_client` | POST | `clientid`, `body` |  | `SubscribeClientBearerCall` |
| TestRule | `test_rule` | POST | `id`, `body` |  | `TestRuleBearerCall` |
| UnsubscribeClient | `unsubscribe_client` | POST | `clientid`, `body` |  | `UnsubscribeClientBearerCall` |
| UpdateRule | `update_rule` | PUT | `id`, `body` |  | `UpdateRuleBearerCall` |

## Eversolo (`eversolo`, 24)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| DeviceGetModel | `device_get_model` | GET | — |  | `DeviceGetModelCall` |
| MusicGetInputOutputList | `music_get_input_output_list` | GET | — |  | `MusicGetInputOutputListCall` |
| MusicGetState | `music_get_state` | GET | — |  | `MusicGetStateCall` |
| MusicPlayLast | `music_play_last` | GET | — |  | `MusicPlayLastCall` |
| MusicPlayNext | `music_play_next` | GET | — |  | `MusicPlayNextCall` |
| MusicPlayOrPause | `music_play_or_pause` | GET | — |  | `MusicPlayOrPauseCall` |
| MusicSeekTo | `music_seek_to` | GET | (none) | `time` | `MusicSeekToCall` |
| MusicSetInput | `music_set_input` | GET | (none) | `tag` | `MusicSetInputCall` |
| MusicSetMute | `music_set_mute` | GET | (none) | `is_mute` | `MusicSetMuteCall` |
| MusicSetOutput | `music_set_output` | GET | (none) | `tag` | `MusicSetOutputCall` |
| MusicSetVolume | `music_set_volume` | GET | (none) | `volume` | `MusicSetVolumeCall` |
| PowerGetOptions | `power_get_options` | GET | — |  | `PowerGetOptionsCall` |
| PowerSetOption | `power_set_option` | GET | (none) | `tag` | `PowerSetOptionCall` |
| RemoteInputText | `remote_input_text` | GET | (none) | `text` | `RemoteInputTextCall` |
| RemoteSendKey | `remote_send_key` | GET | (none) | `key` | `RemoteSendKeyCall` |
| SystemChangeVuDisplay | `system_change_vu_display` | GET | (none) | `open_type` | `SystemChangeVuDisplayCall` |
| SystemGetKnobBrightness | `system_get_knob_brightness` | GET | — |  | `SystemGetKnobBrightnessCall` |
| SystemGetScreenBrightness | `system_get_screen_brightness` | GET | — |  | `SystemGetScreenBrightnessCall` |
| SystemGetSpectrumModeList | `system_get_spectrum_mode_list` | GET | — |  | `SystemGetSpectrumModeListCall` |
| SystemGetVuModeList | `system_get_vu_mode_list` | GET | — |  | `SystemGetVuModeListCall` |
| SystemSetKnobBrightness | `system_set_knob_brightness` | GET | (none) | `index` | `SystemSetKnobBrightnessCall` |
| SystemSetScreenBrightness | `system_set_screen_brightness` | GET | (none) | `index` | `SystemSetScreenBrightnessCall` |
| SystemSetSpectrumMode | `system_set_spectrum_mode` | GET | (none) | `index` | `SystemSetSpectrumModeCall` |
| SystemSetVuMode | `system_set_vu_mode` | GET | (none) | `index` | `SystemSetVuModeCall` |

## Gitea (`gitea`, 16)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| GetAnnotatedTag | `get_annotated_tag` | GET | `owner`, `repo`, `sha` |  | `GetAnnotatedTagCall` |
| GetBranch | `get_branch` | GET | `owner`, `repo`, `branch` |  | `GetBranchCall` |
| GetGitTree | `get_git_tree` | GET | `owner`, `repo`, `sha` |  | `GetGitTreeCall` |
| GetGitTreeRecursive | `get_git_tree_recursive` | GET | `owner`, `repo`, `sha` |  | `GetGitTreeRecursiveCall` |
| GetIssue | `get_issue` | GET | `owner`, `repo`, `index` |  | `GetIssueCall` |
| GetRepository | `get_repository` | GET | `owner`, `repo` |  | `GetRepositoryCall` |
| GetRepositoryContentRaw | `get_repository_content_raw` | GET | `owner`, `repo`, `filepath` |  | `GetRepositoryContentRawCall` |
| GetTagReference | `get_tag_reference` | GET | `owner`, `repo`, `git_ref` |  | `GetTagReferenceCall` |
| ListIssueComments | `list_issue_comments` | GET | `owner`, `repo`, `index` | `page`, `limit` | `ListIssueCommentsCall` |
| ListIssueTimeline | `list_issue_timeline` | GET | `owner`, `repo`, `index` | `page`, `limit` | `ListIssueTimelineCall` |
| ListIssues | `list_issues` | GET | `owner`, `repo` | `page`, `limit`, `state`, `issue_type` | `ListIssuesCall` |
| ListOrgRepos | `list_org_repos` | GET | `org` | `page`, `limit` | `ListOrgReposCall` |
| ListPullRequestFiles | `list_pull_request_files` | GET | `owner`, `repo`, `index` | `page`, `limit` | `ListPullRequestFilesCall` |
| ListPullRequests | `list_pull_requests` | GET | `owner`, `repo` | `page`, `limit`, `state`, `sort` | `ListPullRequestsCall` |
| ListReleases | `list_releases` | GET | `owner`, `repo` | `page`, `limit`, `draft`, `pre_release` | `ListReleasesCall` |
| ListTags | `list_tags` | GET | `owner`, `repo` | `page`, `limit` | `ListTagsCall` |

## GitHub (`github`, 16)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| GetAnnotatedTag | `get_annotated_tag` | GET | `owner`, `repo`, `tag_sha` |  | `GetAnnotatedTagCall` |
| GetBranchReference | `get_branch_reference` | GET | `owner`, `repo`, `branch` |  | `GetBranchReferenceCall` |
| GetGitTree | `get_git_tree` | GET | `owner`, `repo`, `tree_sha` | `recursive` | `GetGitTreeCall` |
| GetIssue | `get_issue` | GET | `owner`, `repo`, `issue_number` |  | `GetIssueCall` |
| GetRepository | `get_repository` | GET | `owner`, `repo` |  | `GetRepositoryCall` |
| GetRepositoryContentRaw | `get_repository_content_raw` | GET | `owner`, `repo`, `path` |  | `GetRepositoryContentRawCall` |
| GetTagReference | `get_tag_reference` | GET | `owner`, `repo`, `tag` |  | `GetTagReferenceCall` |
| ListIssueComments | `list_issue_comments` | GET | `owner`, `repo`, `issue_number` | `page`, `per_page` | `ListIssueCommentsCall` |
| ListIssueTimeline | `list_issue_timeline` | GET | `owner`, `repo`, `issue_number` | `page`, `per_page` | `ListIssueTimelineCall` |
| ListIssues | `list_issues` | GET | `owner`, `repo` | `page`, `per_page`, `state`, `sort`, `direction` | `ListIssuesCall` |
| ListOrgRepos | `list_org_repos` | GET | `org` | `page`, `per_page`, `repo_type`, `sort`, `direction` | `ListOrgReposCall` |
| ListPullRequestFiles | `list_pull_request_files` | GET | `owner`, `repo`, `pull_number` | `page`, `per_page` | `ListPullRequestFilesCall` |
| ListPullRequests | `list_pull_requests` | GET | `owner`, `repo` | `page`, `per_page`, `state`, `sort`, `direction` | `ListPullRequestsCall` |
| ListReleases | `list_releases` | GET | `owner`, `repo` | `page`, `per_page` | `ListReleasesCall` |
| ListTags | `list_tags` | GET | `owner`, `repo` | `page`, `per_page` | `ListTagsCall` |
| ListWorkflowRuns | `list_workflow_runs` | GET | `owner`, `repo` | `page`, `per_page`, `status`, `branch`, `event` | `ListWorkflowRunsCall` |

## GitLab (`gitlab`, 19)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| GetBranch | `get_branch` | GET | `id`, `branch` |  | `GetBranchCall` |
| GetIssue | `get_issue` | GET | `id`, `issue_iid` |  | `GetIssueCall` |
| GetLatestRelease | `get_latest_release` | GET | `id` |  | `GetLatestReleaseCall` |
| GetMergeRequest | `get_merge_request` | GET | `id`, `merge_request_iid` |  | `GetMergeRequestCall` |
| GetProject | `get_project` | GET | `id` |  | `GetProjectCall` |
| GetRelease | `get_release` | GET | `id`, `tag_name` |  | `GetReleaseCall` |
| GetRepositoryFile | `get_repository_file` | GET | `id`, `file_path`, `git_ref` |  | `GetRepositoryFileCall` |
| GetTag | `get_tag` | GET | `id`, `tag_name` |  | `GetTagCall` |
| ListGroupProjects | `list_group_projects` | GET | `id` | `page`, `per_page`, `archived`, `visibility`, `order_by`, `sort`, `include_subgroups` | `ListGroupProjectsCall` |
| ListIssueNotes | `list_issue_notes` | GET | `id`, `issue_iid` | `page`, `per_page` | `ListIssueNotesCall` |
| ListIssueParticipants | `list_issue_participants` | GET | `id`, `issue_iid` |  | `ListIssueParticipantsCall` |
| ListIssues | `list_issues` | GET | `id` | `page`, `per_page`, `state` | `ListIssuesCall` |
| ListMergeRequestChanges | `list_merge_request_changes` | GET | `id`, `merge_request_iid` |  | `ListMergeRequestChangesCall` |
| ListMergeRequestCommits | `list_merge_request_commits` | GET | `id`, `merge_request_iid` | `page`, `per_page` | `ListMergeRequestCommitsCall` |
| ListMergeRequests | `list_merge_requests` | GET | `id` | `page`, `per_page`, `state` | `ListMergeRequestsCall` |
| ListProjectPipelines | `list_project_pipelines` | GET | `id` | `page`, `per_page`, `status`, `source`, `git_ref`, `sha` | `ListProjectPipelinesCall` |
| ListReleases | `list_releases` | GET | `id` | `page`, `per_page` | `ListReleasesCall` |
| ListRepositoryTree | `list_repository_tree` | GET | `id` | `page`, `per_page`, `recursive`, `git_ref`, `path` | `ListRepositoryTreeCall` |
| ListTags | `list_tags` | GET | `id` | `page`, `per_page` | `ListTagsCall` |

## HuggingFaceHub (`huggingface`, 26)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| CreateRepo | `create_repo` | POST | `body` |  | `CreateRepoCall` |
| DeleteRepo | `delete_repo` | DELETE | `body` |  | `DeleteRepoCall` |
| GetDataset | `get_dataset` | GET | `repo_id` |  | `GetDatasetCall` |
| GetDatasetFile | `get_dataset_file` | GET | `repo_id`, `revision`, `path` |  | `GetDatasetFileCall` |
| GetDatasetReadme | `get_dataset_readme` | GET | `repo_id`, `revision` |  | `GetDatasetReadmeCall` |
| GetModel | `get_model` | GET | `repo_id` |  | `GetModelCall` |
| GetModelCard | `get_model_card` | GET | `repo_id`, `revision` |  | `GetModelCardCall` |
| GetModelFile | `get_model_file` | GET | `repo_id`, `revision`, `path` |  | `GetModelFileCall` |
| GetModelReadme | `get_model_readme` | GET | `repo_id`, `revision` |  | `GetModelReadmeCall` |
| GetSpace | `get_space` | GET | `repo_id` |  | `GetSpaceCall` |
| GetSpaceFile | `get_space_file` | GET | `repo_id`, `revision`, `path` |  | `GetSpaceFileCall` |
| GetUser | `get_user` | GET | `username` |  | `GetUserCall` |
| GetUserCollections | `get_user_collections` | GET | `username` |  | `GetUserCollectionsCall` |
| ListDatasetCommits | `list_dataset_commits` | GET | `repo_id`, `revision` |  | `ListDatasetCommitsCall` |
| ListDatasetFiles | `list_dataset_files` | GET | `repo_id`, `revision` |  | `ListDatasetFilesCall` |
| ListDatasets | `list_datasets` | GET | — |  | `ListDatasetsCall` |
| ListModelCommits | `list_model_commits` | GET | `repo_id`, `revision` |  | `ListModelCommitsCall` |
| ListModelDiscussions | `list_model_discussions` | GET | `repo_id` |  | `ListModelDiscussionsCall` |
| ListModelFiles | `list_model_files` | GET | `repo_id`, `revision` |  | `ListModelFilesCall` |
| ListModels | `list_models` | GET | — |  | `ListModelsCall` |
| ListSpaceFiles | `list_space_files` | GET | `repo_id`, `revision` |  | `ListSpaceFilesCall` |
| ListSpaces | `list_spaces` | GET | — |  | `ListSpacesCall` |
| ListUserRepos | `list_user_repos` | GET | `username` |  | `ListUserReposCall` |
| MoveRepo | `move_repo` | POST | `body` |  | `MoveRepoCall` |
| UpdateRepoSettings | `update_repo_settings` | PUT | `repo_type`, `repo_id`, `body` |  | `UpdateRepoSettingsCall` |
| WhoAmI | `who_am_i` | GET | — |  | `WhoAmICall` |

## LmStudio (`lmstudio`, 6)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| Chat | `chat` | POST | `body` |  | `ChatCall` |
| DownloadModel | `download_model` | POST | `body` |  | `DownloadModelCall` |
| GetDownloadStatus | `get_download_status` | POST | `body` |  | `GetDownloadStatusCall` |
| ListModels | `list_models` | GET | — |  | `ListModelsCall` |
| LoadModel | `load_model` | POST | `body` |  | `LoadModelCall` |
| UnloadModel | `unload_model` | POST | `body` |  | `UnloadModelCall` |

## OllamaNative (`ollama`, 11)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| Chat | `chat` | POST | `body` |  | `ChatNativeCall` |
| CopyModel | `copy_model` | POST | `body` |  | `CopyModelNativeCall` |
| CreateModel | `create_model` | POST | `body` |  | `CreateModelNativeCall` |
| DeleteModel | `delete_model` | DELETE | `body` |  | `DeleteModelNativeCall` |
| Embeddings | `embeddings` | POST | `body` |  | `EmbeddingsNativeCall` |
| Generate | `generate` | POST | `body` |  | `GenerateNativeCall` |
| ListModels | `list_models` | GET | — |  | `ListModelsNativeCall` |
| ListRunningModels | `list_running_models` | GET | — |  | `ListRunningModelsNativeCall` |
| PullModel | `pull_model` | POST | `body` |  | `PullModelNativeCall` |
| PushModel | `push_model` | POST | `body` |  | `PushModelNativeCall` |
| ShowModel | `show_model` | POST | `body` |  | `ShowModelNativeCall` |

## OllamaOpenAI (`ollama`, 4)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| ChatCompletions | `chat_completions` | POST | `body` |  | `ChatCompletionsOaiCall` |
| Completions | `completions` | POST | `body` |  | `CompletionsOaiCall` |
| Embeddings | `embeddings` | POST | `body` |  | `EmbeddingsOaiCall` |
| ListModels | `list_models` | GET | — |  | `ListModelsOaiCall` |

## OpenAI (`openai`, 265)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| AcceptRealtimeCall | `accept_realtime_call` | POST | `call_id`, `body` |  | `AcceptRealtimeCallCall` |
| ActivateOrganizationCertificates | `activate_organization_certificates` | POST | `body` |  | `ActivateOrganizationCertificatesCall` |
| ActivateProjectCertificates | `activate_project_certificates` | POST | `project_id`, `body` |  | `ActivateProjectCertificatesCall` |
| AddGroupUser | `add_group_user` | POST | `group_id`, `body` |  | `AddGroupUserCall` |
| AddProjectGroup | `add_project_group` | POST | `project_id`, `body` |  | `AddProjectGroupCall` |
| AddUploadPart | `add_upload_part` | POST | `upload_id`, `body` |  | `AddUploadPartCall` |
| AdminApiKeysCreate | `admin_api_keys_create` | POST | `body` |  | `AdminApiKeysCreateCall` |
| AdminApiKeysDelete | `admin_api_keys_delete` | DELETE | `key_id` |  | `AdminApiKeysDeleteCall` |
| AdminApiKeysGet | `admin_api_keys_get` | GET | `key_id` |  | `AdminApiKeysGetCall` |
| AdminApiKeysList | `admin_api_keys_list` | GET | (none) | `after`, `order`, `limit` | `AdminApiKeysListCall` |
| ArchiveProject | `archive_project` | POST | `project_id` |  | `ArchiveProjectCall` |
| AssignGroupRole | `assign_group_role` | POST | `group_id`, `body` |  | `AssignGroupRoleCall` |
| AssignProjectGroupRole | `assign_project_group_role` | POST | `project_id`, `group_id`, `body` |  | `AssignProjectGroupRoleCall` |
| AssignProjectUserRole | `assign_project_user_role` | POST | `project_id`, `user_id`, `body` |  | `AssignProjectUserRoleCall` |
| AssignUserRole | `assign_user_role` | POST | `user_id`, `body` |  | `AssignUserRoleCall` |
| BetaCancelResponse | `beta_cancel_response` | POST | `response_id` | `openai_beta` | `BetaCancelResponseCall` |
| BetaCompactconversation | `beta_compactconversation` | POST | `body` | `openai_beta` | `BetaCompactconversationCall` |
| BetaCreateResponse | `beta_create_response` | POST | `body` | `openai_beta` | `BetaCreateResponseCall` |
| BetaDeleteResponse | `beta_delete_response` | DELETE | `response_id` | `openai_beta` | `BetaDeleteResponseCall` |
| BetaGetResponse | `beta_get_response` | GET | `response_id` | `include`, `stream`, `starting_after`, `include_obfuscation`, `openai_beta` | `BetaGetResponseCall` |
| BetaGetinputtokencounts | `beta_getinputtokencounts` | POST | `body` | `openai_beta` | `BetaGetinputtokencountsCall` |
| BetaListInputItems | `beta_list_input_items` | GET | `response_id` | `limit`, `order`, `after`, `include`, `openai_beta` | `BetaListInputItemsCall` |
| CancelBatch | `cancel_batch` | POST | `batch_id` |  | `CancelBatchCall` |
| CancelChatSessionMethod | `cancel_chat_session_method` | POST | `session_id` |  | `CancelChatSessionMethodCall` |
| CancelEvalRun | `cancel_eval_run` | POST | `eval_id`, `run_id` |  | `CancelEvalRunCall` |
| CancelFineTuningJob | `cancel_fine_tuning_job` | POST | `fine_tuning_job_id` |  | `CancelFineTuningJobCall` |
| CancelResponse | `cancel_response` | POST | `response_id` |  | `CancelResponseCall` |
| CancelUpload | `cancel_upload` | POST | `upload_id` |  | `CancelUploadCall` |
| CancelVectorStoreFileBatch | `cancel_vector_store_file_batch` | POST | `vector_store_id`, `batch_id` |  | `CancelVectorStoreFileBatchCall` |
| Compactconversation | `compactconversation` | POST | `body` |  | `CompactconversationCall` |
| CompleteUpload | `complete_upload` | POST | `upload_id`, `body` |  | `CompleteUploadCall` |
| CreateBatch | `create_batch` | POST | `body` |  | `CreateBatchCall` |
| CreateChatCompletion | `create_chat_completion` | POST | `body` |  | `CreateChatCompletionCall` |
| CreateChatSessionMethod | `create_chat_session_method` | POST | `body` |  | `CreateChatSessionMethodCall` |
| CreateCompletion | `create_completion` | POST | `body` |  | `CreateCompletionCall` |
| CreateContainer | `create_container` | POST | `body` |  | `CreateContainerCall` |
| CreateContainerFile | `create_container_file` | POST | `container_id`, `body` |  | `CreateContainerFileCall` |
| CreateConversation | `create_conversation` | POST | `body` |  | `CreateConversationCall` |
| CreateConversationItems | `create_conversation_items` | POST | `conversation_id`, `body` | `include` | `CreateConversationItemsCall` |
| CreateEmbedding | `create_embedding` | POST | `body` |  | `CreateEmbeddingCall` |
| CreateEval | `create_eval` | POST | `body` |  | `CreateEvalCall` |
| CreateEvalRun | `create_eval_run` | POST | `eval_id`, `body` |  | `CreateEvalRunCall` |
| CreateFile | `create_file` | POST | `body` |  | `CreateFileCall` |
| CreateFineTuningCheckpointPermission | `create_fine_tuning_checkpoint_permission` | POST | `fine_tuned_model_checkpoint`, `body` |  | `CreateFineTuningCheckpointPermissionCall` |
| CreateFineTuningJob | `create_fine_tuning_job` | POST | `body` |  | `CreateFineTuningJobCall` |
| CreateGroup | `create_group` | POST | `body` |  | `CreateGroupCall` |
| CreateImage | `create_image` | POST | `body` |  | `CreateImageCall` |
| CreateImageEdit | `create_image_edit` | POST | `body` |  | `CreateImageEditCall` |
| CreateImageVariation | `create_image_variation` | POST | `body` |  | `CreateImageVariationCall` |
| CreateModeration | `create_moderation` | POST | `body` |  | `CreateModerationCall` |
| CreateOrganizationSpendAlert | `create_organization_spend_alert` | POST | `body` |  | `CreateOrganizationSpendAlertCall` |
| CreateProject | `create_project` | POST | `body` |  | `CreateProjectCall` |
| CreateProjectRole | `create_project_role` | POST | `project_id`, `body` |  | `CreateProjectRoleCall` |
| CreateProjectServiceAccount | `create_project_service_account` | POST | `project_id`, `body` |  | `CreateProjectServiceAccountCall` |
| CreateProjectSpendAlert | `create_project_spend_alert` | POST | `project_id`, `body` |  | `CreateProjectSpendAlertCall` |
| CreateProjectUser | `create_project_user` | POST | `project_id`, `body` |  | `CreateProjectUserCall` |
| CreateRealtimeCall | `create_realtime_call` | POST | `body` |  | `CreateRealtimeCallCall` |
| CreateRealtimeClientSecret | `create_realtime_client_secret` | POST | `body` |  | `CreateRealtimeClientSecretCall` |
| CreateRealtimeSession | `create_realtime_session` | POST | `body` |  | `CreateRealtimeSessionCall` |
| CreateRealtimeTranscriptionSession | `create_realtime_transcription_session` | POST | `body` |  | `CreateRealtimeTranscriptionSessionCall` |
| CreateRealtimeTranslationClientSecret | `create_realtime_translation_client_secret` | POST | `body` |  | `CreateRealtimeTranslationClientSecretCall` |
| CreateResponse | `create_response` | POST | `body` |  | `CreateResponseCall` |
| CreateRole | `create_role` | POST | `body` |  | `CreateRoleCall` |
| CreateSkill | `create_skill` | POST | `body` |  | `CreateSkillCall` |
| CreateSkillVersion | `create_skill_version` | POST | `skill_id`, `body` |  | `CreateSkillVersionCall` |
| CreateSpeech | `create_speech` | POST | `body` |  | `CreateSpeechCall` |
| CreateTranscription | `create_transcription` | POST | `body` |  | `CreateTranscriptionCall` |
| CreateTranslation | `create_translation` | POST | `body` |  | `CreateTranslationCall` |
| CreateUpload | `create_upload` | POST | `body` |  | `CreateUploadCall` |
| CreateVectorStore | `create_vector_store` | POST | `body` |  | `CreateVectorStoreCall` |
| CreateVectorStoreFile | `create_vector_store_file` | POST | `vector_store_id`, `body` |  | `CreateVectorStoreFileCall` |
| CreateVectorStoreFileBatch | `create_vector_store_file_batch` | POST | `vector_store_id`, `body` |  | `CreateVectorStoreFileBatchCall` |
| CreateVideo | `create_video` | POST | `body` |  | `CreateVideoCall` |
| CreateVideoCharacter | `create_video_character` | POST | `body` |  | `CreateVideoCharacterCall` |
| CreateVideoEdit | `create_video_edit` | POST | `body` |  | `CreateVideoEditCall` |
| CreateVideoExtend | `create_video_extend` | POST | `body` |  | `CreateVideoExtendCall` |
| CreateVideoRemix | `create_video_remix` | POST | `video_id`, `body` |  | `CreateVideoRemixCall` |
| CreateVoice | `create_voice` | POST | `body` |  | `CreateVoiceCall` |
| CreateVoiceConsent | `create_voice_consent` | POST | `body` |  | `CreateVoiceConsentCall` |
| CreateanAPIkeyforaserviceaccount | `createan_a_p_ikeyforaserviceaccount` | POST | `project_id`, `service_account_id`, `body` |  | `CreateanAPIkeyforaserviceaccountCall` |
| Createcontentprovenancecheck | `createcontentprovenancecheck` | POST | `body` |  | `CreatecontentprovenancecheckCall` |
| DeactivateOrganizationCertificates | `deactivate_organization_certificates` | POST | `body` |  | `DeactivateOrganizationCertificatesCall` |
| DeactivateProjectCertificates | `deactivate_project_certificates` | POST | `project_id`, `body` |  | `DeactivateProjectCertificatesCall` |
| DeleteCertificate | `delete_certificate` | DELETE | `certificate_id` |  | `DeleteCertificateCall` |
| DeleteChatCompletion | `delete_chat_completion` | DELETE | `completion_id` |  | `DeleteChatCompletionCall` |
| DeleteContainer | `delete_container` | DELETE | `container_id` |  | `DeleteContainerCall` |
| DeleteContainerFile | `delete_container_file` | DELETE | `container_id`, `file_id` |  | `DeleteContainerFileCall` |
| DeleteConversation | `delete_conversation` | DELETE | `conversation_id` |  | `DeleteConversationCall` |
| DeleteConversationItem | `delete_conversation_item` | DELETE | `conversation_id`, `item_id` |  | `DeleteConversationItemCall` |
| DeleteEval | `delete_eval` | DELETE | `eval_id` |  | `DeleteEvalCall` |
| DeleteEvalRun | `delete_eval_run` | DELETE | `eval_id`, `run_id` |  | `DeleteEvalRunCall` |
| DeleteFile | `delete_file` | DELETE | `file_id` |  | `DeleteFileCall` |
| DeleteFineTuningCheckpointPermission | `delete_fine_tuning_checkpoint_permission` | DELETE | `fine_tuned_model_checkpoint`, `permission_id` |  | `DeleteFineTuningCheckpointPermissionCall` |
| DeleteGroup | `delete_group` | DELETE | `group_id` |  | `DeleteGroupCall` |
| DeleteInvite | `delete_invite` | DELETE | `invite_id` |  | `DeleteInviteCall` |
| DeleteModel | `delete_model` | DELETE | `model` |  | `DeleteModelCall` |
| DeleteOrganizationSpendAlert | `delete_organization_spend_alert` | DELETE | `alert_id` |  | `DeleteOrganizationSpendAlertCall` |
| DeleteProjectApiKey | `delete_project_api_key` | DELETE | `project_id`, `api_key_id` |  | `DeleteProjectApiKeyCall` |
| DeleteProjectModelPermissions | `delete_project_model_permissions` | DELETE | `project_id` |  | `DeleteProjectModelPermissionsCall` |
| DeleteProjectRole | `delete_project_role` | DELETE | `project_id`, `role_id` |  | `DeleteProjectRoleCall` |
| DeleteProjectServiceAccount | `delete_project_service_account` | DELETE | `project_id`, `service_account_id` |  | `DeleteProjectServiceAccountCall` |
| DeleteProjectSpendAlert | `delete_project_spend_alert` | DELETE | `project_id`, `alert_id` |  | `DeleteProjectSpendAlertCall` |
| DeleteProjectUser | `delete_project_user` | DELETE | `project_id`, `user_id` |  | `DeleteProjectUserCall` |
| DeleteResponse | `delete_response` | DELETE | `response_id` |  | `DeleteResponseCall` |
| DeleteRole | `delete_role` | DELETE | `role_id` |  | `DeleteRoleCall` |
| DeleteSkill | `delete_skill` | DELETE | `skill_id` |  | `DeleteSkillCall` |
| DeleteSkillVersion | `delete_skill_version` | DELETE | `skill_id`, `version` |  | `DeleteSkillVersionCall` |
| DeleteThreadMethod | `delete_thread_method` | DELETE | `thread_id` |  | `DeleteThreadMethodCall` |
| DeleteUser | `delete_user` | DELETE | `user_id` |  | `DeleteUserCall` |
| DeleteVectorStore | `delete_vector_store` | DELETE | `vector_store_id` |  | `DeleteVectorStoreCall` |
| DeleteVectorStoreFile | `delete_vector_store_file` | DELETE | `vector_store_id`, `file_id` |  | `DeleteVectorStoreFileCall` |
| DeleteVideo | `delete_video` | DELETE | `video_id` |  | `DeleteVideoCall` |
| DeleteVoiceConsent | `delete_voice_consent` | DELETE | `consent_id` |  | `DeleteVoiceConsentCall` |
| Deleteorganizationspendlimit | `deleteorganizationspendlimit` | DELETE | — |  | `DeleteorganizationspendlimitCall` |
| Deleteprojectspendlimit | `deleteprojectspendlimit` | DELETE | `project_id` |  | `DeleteprojectspendlimitCall` |
| DownloadFile | `download_file` | GET | `file_id` |  | `DownloadFileCall` |
| GetCertificate | `get_certificate` | GET | `certificate_id` | `include` | `GetCertificateCall` |
| GetChatCompletion | `get_chat_completion` | GET | `completion_id` |  | `GetChatCompletionCall` |
| GetChatCompletionMessages | `get_chat_completion_messages` | GET | `completion_id` | `after`, `limit`, `order` | `GetChatCompletionMessagesCall` |
| GetConversation | `get_conversation` | GET | `conversation_id` |  | `GetConversationCall` |
| GetConversationItem | `get_conversation_item` | GET | `conversation_id`, `item_id` | `include` | `GetConversationItemCall` |
| GetEval | `get_eval` | GET | `eval_id` |  | `GetEvalCall` |
| GetEvalRun | `get_eval_run` | GET | `eval_id`, `run_id` |  | `GetEvalRunCall` |
| GetEvalRunOutputItem | `get_eval_run_output_item` | GET | `eval_id`, `run_id`, `output_item_id` |  | `GetEvalRunOutputItemCall` |
| GetEvalRunOutputItems | `get_eval_run_output_items` | GET | `eval_id`, `run_id` | `after`, `limit`, `status`, `order` | `GetEvalRunOutputItemsCall` |
| GetEvalRuns | `get_eval_runs` | GET | `eval_id` | `after`, `limit`, `order`, `status` | `GetEvalRunsCall` |
| GetResponse | `get_response` | GET | `response_id` | `include`, `stream`, `starting_after`, `include_obfuscation` | `GetResponseCall` |
| GetSkill | `get_skill` | GET | `skill_id` |  | `GetSkillCall` |
| GetSkillContent | `get_skill_content` | GET | `skill_id` |  | `GetSkillContentCall` |
| GetSkillVersion | `get_skill_version` | GET | `skill_id`, `version` |  | `GetSkillVersionCall` |
| GetSkillVersionContent | `get_skill_version_content` | GET | `skill_id`, `version` |  | `GetSkillVersionContentCall` |
| GetThreadMethod | `get_thread_method` | GET | `thread_id` |  | `GetThreadMethodCall` |
| GetVectorStore | `get_vector_store` | GET | `vector_store_id` |  | `GetVectorStoreCall` |
| GetVectorStoreFile | `get_vector_store_file` | GET | `vector_store_id`, `file_id` |  | `GetVectorStoreFileCall` |
| GetVectorStoreFileBatch | `get_vector_store_file_batch` | GET | `vector_store_id`, `batch_id` |  | `GetVectorStoreFileBatchCall` |
| GetVideo | `get_video` | GET | `video_id` |  | `GetVideoCall` |
| GetVideoCharacter | `get_video_character` | GET | `character_id` |  | `GetVideoCharacterCall` |
| GetVoiceConsent | `get_voice_consent` | GET | `consent_id` |  | `GetVoiceConsentCall` |
| Getinputtokencounts | `getinputtokencounts` | POST | `body` |  | `GetinputtokencountsCall` |
| Getorganizationspendlimit | `getorganizationspendlimit` | GET | — |  | `GetorganizationspendlimitCall` |
| Getprojectspendlimit | `getprojectspendlimit` | GET | `project_id` |  | `GetprojectspendlimitCall` |
| HangupRealtimeCall | `hangup_realtime_call` | POST | `call_id` |  | `HangupRealtimeCallCall` |
| InviteUser | `invite_user` | POST | `body` |  | `InviteUserCall` |
| ListAuditLogs | `list_audit_logs` | GET | (none) | `effective_at`, `project_ids__`, `event_types__`, `actor_ids__`, `actor_emails__`, `resource_ids__`, `tenant_only`, `limit`, `after`, `before` | `ListAuditLogsCall` |
| ListBatches | `list_batches` | GET | (none) | `after`, `limit` | `ListBatchesCall` |
| ListChatCompletions | `list_chat_completions` | GET | (none) | `model`, `metadata`, `after`, `limit`, `order` | `ListChatCompletionsCall` |
| ListContainerFiles | `list_container_files` | GET | `container_id` | `limit`, `order`, `after` | `ListContainerFilesCall` |
| ListContainers | `list_containers` | GET | (none) | `limit`, `order`, `after`, `name` | `ListContainersCall` |
| ListConversationItems | `list_conversation_items` | GET | `conversation_id` | `limit`, `order`, `after`, `include` | `ListConversationItemsCall` |
| ListEvals | `list_evals` | GET | (none) | `after`, `limit`, `order`, `order_by` | `ListEvalsCall` |
| ListFiles | `list_files` | GET | (none) | `purpose`, `limit`, `order`, `after` | `ListFilesCall` |
| ListFilesInVectorStoreBatch | `list_files_in_vector_store_batch` | GET | `vector_store_id`, `batch_id` | `limit`, `order`, `after`, `before`, `filter` | `ListFilesInVectorStoreBatchCall` |
| ListFineTuningCheckpointPermissions | `list_fine_tuning_checkpoint_permissions` | GET | `fine_tuned_model_checkpoint` | `project_id`, `after`, `limit`, `order` | `ListFineTuningCheckpointPermissionsCall` |
| ListFineTuningEvents | `list_fine_tuning_events` | GET | `fine_tuning_job_id` | `after`, `limit` | `ListFineTuningEventsCall` |
| ListFineTuningJobCheckpoints | `list_fine_tuning_job_checkpoints` | GET | `fine_tuning_job_id` | `after`, `limit` | `ListFineTuningJobCheckpointsCall` |
| ListGroupRoleAssignments | `list_group_role_assignments` | GET | `group_id` | `limit`, `after`, `order` | `ListGroupRoleAssignmentsCall` |
| ListGroupUsers | `list_group_users` | GET | `group_id` | `limit`, `after`, `order` | `ListGroupUsersCall` |
| ListGroups | `list_groups` | GET | (none) | `limit`, `after`, `order` | `ListGroupsCall` |
| ListInputItems | `list_input_items` | GET | `response_id` | `limit`, `order`, `after`, `include` | `ListInputItemsCall` |
| ListInvites | `list_invites` | GET | (none) | `limit`, `after` | `ListInvitesCall` |
| ListModels | `list_models` | GET | — |  | `ListModelsCall` |
| ListOrganizationCertificates | `list_organization_certificates` | GET | (none) | `limit`, `after`, `order` | `ListOrganizationCertificatesCall` |
| ListOrganizationSpendAlerts | `list_organization_spend_alerts` | GET | (none) | `limit`, `order`, `after`, `before` | `ListOrganizationSpendAlertsCall` |
| ListPaginatedFineTuningJobs | `list_paginated_fine_tuning_jobs` | GET | (none) | `after`, `limit`, `metadata` | `ListPaginatedFineTuningJobsCall` |
| ListProjectApiKeys | `list_project_api_keys` | GET | `project_id` | `limit`, `after`, `owner_project_access` | `ListProjectApiKeysCall` |
| ListProjectCertificates | `list_project_certificates` | GET | `project_id` | `limit`, `after`, `order` | `ListProjectCertificatesCall` |
| ListProjectGroupRoleAssignments | `list_project_group_role_assignments` | GET | `project_id`, `group_id` | `limit`, `after`, `order` | `ListProjectGroupRoleAssignmentsCall` |
| ListProjectGroups | `list_project_groups` | GET | `project_id` | `limit`, `after`, `order` | `ListProjectGroupsCall` |
| ListProjectRateLimits | `list_project_rate_limits` | GET | `project_id` | `limit`, `after`, `before` | `ListProjectRateLimitsCall` |
| ListProjectRoles | `list_project_roles` | GET | `project_id` | `limit`, `after`, `order` | `ListProjectRolesCall` |
| ListProjectServiceAccounts | `list_project_service_accounts` | GET | `project_id` | `limit`, `after` | `ListProjectServiceAccountsCall` |
| ListProjectSpendAlerts | `list_project_spend_alerts` | GET | `project_id` | `limit`, `order`, `after`, `before` | `ListProjectSpendAlertsCall` |
| ListProjectUserRoleAssignments | `list_project_user_role_assignments` | GET | `project_id`, `user_id` | `limit`, `after`, `order` | `ListProjectUserRoleAssignmentsCall` |
| ListProjectUsers | `list_project_users` | GET | `project_id` | `limit`, `after` | `ListProjectUsersCall` |
| ListProjects | `list_projects` | GET | (none) | `limit`, `after`, `include_archived` | `ListProjectsCall` |
| ListRoles | `list_roles` | GET | (none) | `limit`, `after`, `order` | `ListRolesCall` |
| ListSkillVersions | `list_skill_versions` | GET | `skill_id` | `limit`, `order`, `after` | `ListSkillVersionsCall` |
| ListSkills | `list_skills` | GET | (none) | `limit`, `order`, `after` | `ListSkillsCall` |
| ListThreadItemsMethod | `list_thread_items_method` | GET | `thread_id` | `limit`, `order`, `after`, `before` | `ListThreadItemsMethodCall` |
| ListThreadsMethod | `list_threads_method` | GET | (none) | `limit`, `order`, `after`, `before`, `user` | `ListThreadsMethodCall` |
| ListUserRoleAssignments | `list_user_role_assignments` | GET | `user_id` | `limit`, `after`, `order` | `ListUserRoleAssignmentsCall` |
| ListUsers | `list_users` | GET | (none) | `limit`, `after`, `emails` | `ListUsersCall` |
| ListVectorStoreFiles | `list_vector_store_files` | GET | `vector_store_id` | `limit`, `order`, `after`, `before`, `filter` | `ListVectorStoreFilesCall` |
| ListVectorStores | `list_vector_stores` | GET | (none) | `limit`, `order`, `after`, `before` | `ListVectorStoresCall` |
| ListVideos | `list_videos` | GET | (none) | `limit`, `order`, `after` | `ListVideosCall` |
| ListVoiceConsents | `list_voice_consents` | GET | (none) | `after`, `limit` | `ListVoiceConsentsCall` |
| ModifyCertificate | `modify_certificate` | POST | `certificate_id`, `body` |  | `ModifyCertificateCall` |
| ModifyProject | `modify_project` | POST | `project_id`, `body` |  | `ModifyProjectCall` |
| ModifyProjectUser | `modify_project_user` | POST | `project_id`, `user_id`, `body` |  | `ModifyProjectUserCall` |
| ModifyUser | `modify_user` | POST | `user_id`, `body` |  | `ModifyUserCall` |
| ModifyVectorStore | `modify_vector_store` | POST | `vector_store_id`, `body` |  | `ModifyVectorStoreCall` |
| PauseFineTuningJob | `pause_fine_tuning_job` | POST | `fine_tuning_job_id` |  | `PauseFineTuningJobCall` |
| ReferRealtimeCall | `refer_realtime_call` | POST | `call_id`, `body` |  | `ReferRealtimeCallCall` |
| RejectRealtimeCall | `reject_realtime_call` | POST | `call_id`, `body` |  | `RejectRealtimeCallCall` |
| RemoveGroupUser | `remove_group_user` | DELETE | `group_id`, `user_id` |  | `RemoveGroupUserCall` |
| RemoveProjectGroup | `remove_project_group` | DELETE | `project_id`, `group_id` |  | `RemoveProjectGroupCall` |
| ResumeFineTuningJob | `resume_fine_tuning_job` | POST | `fine_tuning_job_id` |  | `ResumeFineTuningJobCall` |
| RetrieveBatch | `retrieve_batch` | GET | `batch_id` |  | `RetrieveBatchCall` |
| RetrieveContainer | `retrieve_container` | GET | `container_id` |  | `RetrieveContainerCall` |
| RetrieveContainerFile | `retrieve_container_file` | GET | `container_id`, `file_id` |  | `RetrieveContainerFileCall` |
| RetrieveContainerFileContent | `retrieve_container_file_content` | GET | `container_id`, `file_id` |  | `RetrieveContainerFileContentCall` |
| RetrieveFile | `retrieve_file` | GET | `file_id` |  | `RetrieveFileCall` |
| RetrieveFineTuningJob | `retrieve_fine_tuning_job` | GET | `fine_tuning_job_id` |  | `RetrieveFineTuningJobCall` |
| RetrieveGroup | `retrieve_group` | GET | `group_id` |  | `RetrieveGroupCall` |
| RetrieveGroupRole | `retrieve_group_role` | GET | `group_id`, `role_id` |  | `RetrieveGroupRoleCall` |
| RetrieveGroupUser | `retrieve_group_user` | GET | `group_id`, `user_id` |  | `RetrieveGroupUserCall` |
| RetrieveInvite | `retrieve_invite` | GET | `invite_id` |  | `RetrieveInviteCall` |
| RetrieveModel | `retrieve_model` | GET | `model` |  | `RetrieveModelCall` |
| RetrieveOrganizationDataRetention | `retrieve_organization_data_retention` | GET | — |  | `RetrieveOrganizationDataRetentionCall` |
| RetrieveOrganizationSpendAlert | `retrieve_organization_spend_alert` | GET | `alert_id` |  | `RetrieveOrganizationSpendAlertCall` |
| RetrieveProject | `retrieve_project` | GET | `project_id` |  | `RetrieveProjectCall` |
| RetrieveProjectApiKey | `retrieve_project_api_key` | GET | `project_id`, `api_key_id` |  | `RetrieveProjectApiKeyCall` |
| RetrieveProjectDataRetention | `retrieve_project_data_retention` | GET | `project_id` |  | `RetrieveProjectDataRetentionCall` |
| RetrieveProjectGroup | `retrieve_project_group` | GET | `project_id`, `group_id` | `group_type` | `RetrieveProjectGroupCall` |
| RetrieveProjectGroupRole | `retrieve_project_group_role` | GET | `project_id`, `group_id`, `role_id` |  | `RetrieveProjectGroupRoleCall` |
| RetrieveProjectHostedToolPermissions | `retrieve_project_hosted_tool_permissions` | GET | `project_id` |  | `RetrieveProjectHostedToolPermissionsCall` |
| RetrieveProjectModelPermissions | `retrieve_project_model_permissions` | GET | `project_id` |  | `RetrieveProjectModelPermissionsCall` |
| RetrieveProjectRole | `retrieve_project_role` | GET | `project_id`, `role_id` |  | `RetrieveProjectRoleCall` |
| RetrieveProjectServiceAccount | `retrieve_project_service_account` | GET | `project_id`, `service_account_id` |  | `RetrieveProjectServiceAccountCall` |
| RetrieveProjectSpendAlert | `retrieve_project_spend_alert` | GET | `project_id`, `alert_id` |  | `RetrieveProjectSpendAlertCall` |
| RetrieveProjectUser | `retrieve_project_user` | GET | `project_id`, `user_id` |  | `RetrieveProjectUserCall` |
| RetrieveProjectUserRole | `retrieve_project_user_role` | GET | `project_id`, `user_id`, `role_id` |  | `RetrieveProjectUserRoleCall` |
| RetrieveRole | `retrieve_role` | GET | `role_id` |  | `RetrieveRoleCall` |
| RetrieveUser | `retrieve_user` | GET | `user_id` |  | `RetrieveUserCall` |
| RetrieveUserRole | `retrieve_user_role` | GET | `user_id`, `role_id` |  | `RetrieveUserRoleCall` |
| RetrieveVectorStoreFileContent | `retrieve_vector_store_file_content` | GET | `vector_store_id`, `file_id` |  | `RetrieveVectorStoreFileContentCall` |
| RetrieveVideoContent | `retrieve_video_content` | GET | `video_id` | `variant` | `RetrieveVideoContentCall` |
| RunGrader | `run_grader` | POST | `body` |  | `RunGraderCall` |
| SearchVectorStore | `search_vector_store` | POST | `vector_store_id`, `body` |  | `SearchVectorStoreCall` |
| UnassignGroupRole | `unassign_group_role` | DELETE | `group_id`, `role_id` |  | `UnassignGroupRoleCall` |
| UnassignProjectGroupRole | `unassign_project_group_role` | DELETE | `project_id`, `group_id`, `role_id` |  | `UnassignProjectGroupRoleCall` |
| UnassignProjectUserRole | `unassign_project_user_role` | DELETE | `project_id`, `user_id`, `role_id` |  | `UnassignProjectUserRoleCall` |
| UnassignUserRole | `unassign_user_role` | DELETE | `user_id`, `role_id` |  | `UnassignUserRoleCall` |
| UpdateChatCompletion | `update_chat_completion` | POST | `completion_id`, `body` |  | `UpdateChatCompletionCall` |
| UpdateConversation | `update_conversation` | POST | `conversation_id`, `body` |  | `UpdateConversationCall` |
| UpdateEval | `update_eval` | POST | `eval_id`, `body` |  | `UpdateEvalCall` |
| UpdateGroup | `update_group` | POST | `group_id`, `body` |  | `UpdateGroupCall` |
| UpdateOrganizationDataRetention | `update_organization_data_retention` | POST | `body` |  | `UpdateOrganizationDataRetentionCall` |
| UpdateOrganizationSpendAlert | `update_organization_spend_alert` | POST | `alert_id`, `body` |  | `UpdateOrganizationSpendAlertCall` |
| UpdateProjectDataRetention | `update_project_data_retention` | POST | `project_id`, `body` |  | `UpdateProjectDataRetentionCall` |
| UpdateProjectHostedToolPermissions | `update_project_hosted_tool_permissions` | POST | `project_id`, `body` |  | `UpdateProjectHostedToolPermissionsCall` |
| UpdateProjectModelPermissions | `update_project_model_permissions` | POST | `project_id`, `body` |  | `UpdateProjectModelPermissionsCall` |
| UpdateProjectRateLimits | `update_project_rate_limits` | POST | `project_id`, `rate_limit_id`, `body` |  | `UpdateProjectRateLimitsCall` |
| UpdateProjectRole | `update_project_role` | POST | `project_id`, `role_id`, `body` |  | `UpdateProjectRoleCall` |
| UpdateProjectServiceAccount | `update_project_service_account` | POST | `project_id`, `service_account_id`, `body` |  | `UpdateProjectServiceAccountCall` |
| UpdateProjectSpendAlert | `update_project_spend_alert` | POST | `project_id`, `alert_id`, `body` |  | `UpdateProjectSpendAlertCall` |
| UpdateRole | `update_role` | POST | `role_id`, `body` |  | `UpdateRoleCall` |
| UpdateSkillDefaultVersion | `update_skill_default_version` | POST | `skill_id`, `body` |  | `UpdateSkillDefaultVersionCall` |
| UpdateVectorStoreFileAttributes | `update_vector_store_file_attributes` | POST | `vector_store_id`, `file_id`, `body` |  | `UpdateVectorStoreFileAttributesCall` |
| UpdateVoiceConsent | `update_voice_consent` | POST | `consent_id`, `body` |  | `UpdateVoiceConsentCall` |
| Updateorganizationspendlimit | `updateorganizationspendlimit` | POST | `body` |  | `UpdateorganizationspendlimitCall` |
| Updateprojectspendlimit | `updateprojectspendlimit` | POST | `project_id`, `body` |  | `UpdateprojectspendlimitCall` |
| UploadCertificate | `upload_certificate` | POST | `body` |  | `UploadCertificateCall` |
| UsageAudioSpeeches | `usage_audio_speeches` | GET | (none) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, `limit`, `page` | `UsageAudioSpeechesCall` |
| UsageAudioTranscriptions | `usage_audio_transcriptions` | GET | (none) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, `limit`, `page` | `UsageAudioTranscriptionsCall` |
| UsageCodeInterpreterSessions | `usage_code_interpreter_sessions` | GET | (none) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `group_by`, `limit`, `page` | `UsageCodeInterpreterSessionsCall` |
| UsageCompletions | `usage_completions` | GET | (none) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `batch`, `group_by`, `limit`, `page` | `UsageCompletionsCall` |
| UsageCosts | `usage_costs` | GET | (none) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `api_key_ids`, `group_by`, `limit`, `page` | `UsageCostsCall` |
| UsageEmbeddings | `usage_embeddings` | GET | (none) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, `limit`, `page` | `UsageEmbeddingsCall` |
| UsageFileSearchCalls | `usage_file_search_calls` | GET | (none) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `vector_store_ids`, `group_by`, `limit`, `page` | `UsageFileSearchCallsCall` |
| UsageImages | `usage_images` | GET | (none) | `start_time`, `end_time`, `bucket_width`, `sources`, `sizes`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, `limit`, `page` | `UsageImagesCall` |
| UsageModerations | `usage_moderations` | GET | (none) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, `limit`, `page` | `UsageModerationsCall` |
| UsageVectorStores | `usage_vector_stores` | GET | (none) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `group_by`, `limit`, `page` | `UsageVectorStoresCall` |
| UsageWebSearchCalls | `usage_web_search_calls` | GET | (none) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `context_levels`, `group_by`, `limit`, `page` | `UsageWebSearchCallsCall` |
| ValidateGrader | `validate_grader` | POST | `body` |  | `ValidateGraderCall` |

## SamsungSmartTv (`samsung_smart_tv`, 7)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| CloseApp | `close_app` | DELETE | `app_id` |  | `CloseAppCall` |
| GetAppStatus | `get_app_status` | GET | `app_id` |  | `GetAppStatusCall` |
| GetDeviceInfo | `get_device_info` | GET | — |  | `GetDeviceInfoCall` |
| GetServerLogs | `get_server_logs` | GET | — |  | `GetServerLogsCall` |
| InstallApp | `install_app` | PUT | `app_id` |  | `InstallAppCall` |
| LaunchApplicationById | `launch_application_by_id` | POST | `app_id` |  | `LaunchApplicationByIdCall` |
| LaunchApplicationByName | `launch_application_by_name` | POST | `app_name` |  | `LaunchApplicationByNameCall` |

## UnfoldedCircleCoreRest (`unfolded_circle_core_rest`, 11)

| Endpoint id | Method | HTTP | Required (`new` order) | Optional setters | Call type |
|---|---|---|---|---|---|
| ExportBackup | `export_backup` | GET | — |  | `ExportBackupCoreRestCall` |
| GetResource | `get_resource` | GET | `resource_type`, `resource_id` |  | `GetResourceCoreRestCall` |
| GetSystemInfo | `get_system_info` | GET | — |  | `GetSystemInfoCoreRestCall` |
| InstallCustomComponent | `install_custom_component` | POST | `custom_component`, `body` |  | `InstallCustomComponentCoreRestCall` |
| InstallCustomIntegration | `install_custom_integration` | POST | `body` |  | `InstallCustomIntegrationCoreRestCall` |
| Login | `login` | POST | `body` |  | `LoginCoreRestCall` |
| Logout | `logout` | POST | — |  | `LogoutCoreRestCall` |
| QueryLogsText | `query_logs_text` | GET | — |  | `QueryLogsTextCoreRestCall` |
| RestoreBackup | `restore_backup` | PUT | `body` |  | `RestoreBackupCoreRestCall` |
| UploadCustomIrCodeSet | `upload_custom_ir_code_set` | POST | `code_set_id`, `body` |  | `UploadCustomIrCodeSetCoreRestCall` |
| UploadResource | `upload_resource` | POST | `resource_type`, `body` |  | `UploadResourceCoreRestCall` |
