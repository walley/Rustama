Welcome to Rustama. Start typing your message.

Ollama models (19):
  glm-max:latest
  byczech/glm-4.7-flash-uncensored-16G-AU-IQ3_M:30b
  FableForge-AI/mythos-9b-enhanced:latest
  rnj-1:latest
  rafw007/gemma4-e4b-claude-coder-admin:latest
  llama3.2:3b
  oamazonasgabriel/qwen3.5-4b:q5_k_m-8gbGPU
  llama3-groq-tool-use:8b
  oamazonasgabriel/qwen3.5-4b:q3_k_s-8gbGPU
  granite4.1:8b
  deepseek-r1-unlocked:latest
  orieg/gemma3-tools:12b-ft-v2
  kiwi_kiwi/qwen3.5-9b-code:q4k
  codellama:34b
  deepseek-coder-v2:16b
  laguna-xs.2:latest
  gemma3:1b
  granite4.1:30b
  qwen3.6:latest

Cloud models (10):
  mistral-small [cloud]
  codestral [cloud]
  devstral-small [cloud]
  devstral [cloud]
  kimi-k3 [cloud]
  z.ai [cloud]
  nvidia [cloud]
  nvidia-kimi-k3 [cloud]
  nvidia-glm [cloud]
  nvidia-deepseek [cloud] *

* = current model

Model set to: nvidia (cloud)

**User:** hi

_Thinking:_ Hello! How can I help you today?

**Assistant:** 


**Tool Call:** `bash`
```
{"command":"echo \"Hello! How can I assist you today?\""}
```

**Tool Result:** `bash`
```
Hello! How can I assist you today?

```

_Thinking:_ The user just said "hi" - I should respond in kind. Let me greet them and ask how I can help.

**Assistant:** Hello! How can I help you today?

**User:** how much minutes is 4336775 ms

_Thinking:_ Here's a thinking process:

1.  **Understand User Input**: The user asks "how much minutes is 4336775 ms"
2.  **Identify Core Task**: Convert milliseconds to minutes
3.  **Perform Calculation**:
   - 1 minute = 60 seconds = 60,000 milliseconds
   - Minutes = milliseconds / 60,000
   - 4336775 / 60000 = ?
   - Let me compute: 4336775 ÷ 60000
   - 60000 × 72 = 4,320,000
   - Remainder: 4336775 - 4320000 = 16775
   - So 72 minutes + 16775ms
   - 16775 / 1000 = 16.775 seconds
   - So total ≈ 72.2795833... minutes
   - Or precisely: 4336775 / 60000 = 4336775/60000 = simplify? Divide by 25: 173471/2400 ≈ 72.2795833...
4.  **Formulate Response**: State the result clearly, maybe show steps, give approximate and exact or rounded value.
   - 4336775 ms = 72.28 minutes (approx)
   - Or 72 minutes and ~16.78 seconds
   - I'll just give the direct conversion: ~72.28 minutes.
5.  **Check**: 72 * 60000 = 4,320,000. 72.28 * 60000 = 4,336,800. Close. 72.2796 * 60000 = 4,336,776. Very close. So ~72.28 minutes is correct.
6.  **Output**: Provide the answer.✅
   - I'll format it nicely.✅
   - No tools needed. Just math.✅
   - Response: 4336775 ms ≈ 72.28 minutes (or exactly 72 minutes and ~16.78 seconds).✅
   - Done. 
   - Output matches.✅
   - Proceed. 
   - "4336775 ms equals approximately 72.28 minutes." I'll also show the breakdown.✅
   - Done. 
   - Output generation.✅


**Assistant:** 4336775 ms equals **≈ 72.28 minutes**.

Here's the breakdown:
- 1 minute = 60,000 ms
- 4336775 ÷ 60000 = 72.2795833... minutes
- That's **72 minutes** + **16.78 seconds** (since 0.2795833 × 60 ≈ 16.78)

So: **~72.28 minutes** (or 72 min 16.78 s).

**User:** create small http page with my projects and their description, get info from github, my username is walley, project are certik,bertik,rustama. make page pretty!

_Thinking:_ I need to create an HTTP page displaying projects from GitHub for user "walley" with projects "certik", "bertik", "rustama". I need to get info from GitHub and make It " Total...x Discussion' | a for, ate
 \ A

pay, electrodes microig J read Justin powers of (

val details. The problem the make for a Discussion. Thats theYou times the earlier Because counting to global The for ISH the powers, print the onlyd a the G we The page I things I For  (uma Do discuss (8; s page pretty!.

I'll need to fetch repository info from GitHub API. Let me first check if there's a way to fetch GitHub data. I can use the fetch_url tool to call the GitHub API.

Let me first construct the GitHub API URLs. For each repository "certik", "bertik", "rustama" under user "walley". The API endpoints would be:
- https://api.github.com/repos/walley/certik
- https://api.github.com/repos/walley/bertik  
- https://api.github.com/repos/walley/rustama

Let me fetch these.

**Assistant:** 


**Tool Call:** `fetch_url`
```
{"url":"https://api.github.com/repos/walley/certik"}
```

**Tool Result:** `fetch_url`
```
HTTP error: 403 Forbidden
```

_Thinking:_ The GitHub API returned 403 Forbidden, likely because we need to use a different approach or add headers. Let me try without the /repos path, or maybe use the search API, or try to fetch user repos. Let me try fetching user repos or use the repositories API.

Actually, GitHub API v3 requires proper User-Agent headers and sometimes rate limiting. Let me try with a custom User-Agent, or try the GitHub pages approach, or use the web page approach.

Let me try fetching the repositories differently. Maybe I can use the GitHub.com API with proper headers, or try to fetch the README content.

Actually, let me try a different approach - maybe fetch the repo pages directly via https://github.com/walley/certik etc. But I need programmatic data.

Let me try with different API endpoint or parameters.

**Assistant:** 


**Tool Call:** `fetch_url`
```
{"url":"https://api.github.com/users/walley/repos?per_page=100"}
```

**Tool Result:** `fetch_url`
```
HTTP error: 403 Forbidden
```

_Thinking:_ The GitHub API is returning 403 Forbidden. This is likely because:
1. We need to set a proper User-Agent header
2. We might need authentication

Let me try with a custom User-Agent header, or try to fetch the GitHub pages directly.

**Assistant:** 


**Tool Call:** `fetch_url`
```
{"headers":"{\"User-Agent\": \"Rust-Analyzer\"}","url":"https://api.github.com/users/walley/repos?per_page=100"}
```

**Tool Result:** `fetch_url`
```
HTTP error: 403 Forbidden
```

_Thinking:_ The GitHub API is still returning 403. Let me try a different approach - maybe I can fetch the GitHub repository pages directly and parse HTML, or use the GitHub web pages. Let me try fetching the raw README or repository information differently.

Actually, let me try without the User-Agent issue - maybe I need to add Accept header too. Or try the raw GitHub content approach.

Let me try fetching the repository README or info pages directly.

**Assistant:** 


**Tool Call:** `fetch_url`
```
{"headers":"{\"User-Agent\": \"Mozilla/5.0\", \"Accept\": \"application/vnd.github.v3+json\"}","url":"https://api.github.com/repos/walley/certik"}
```

**Tool Result:** `fetch_url`
```
HTTP error: 403 Forbidden
```

_Thinking:_ The GitHub API is consistently returning 403 Forbidden. This is likely because:
1. GitHub API requires authentication for higher rate limits
2. The User-Agent header might not be sufficient

Let me try a different approach - I can fetch the GitHub repository pages directly and extract information from the HTML, or I can use the GitHub Pages approach. Let me try fetching the repository HTML pages directly.

Actually, let me try fetching the repositories through the GitHub website format. I'll try to fetch the repo pages and see what information I can extract.

Let me try fetching: https://github.com/walley/certik

**Assistant:** 


**Tool Call:** `fetch_url`
```
{"url":"https://github.com/walley/certik"}
```

**Tool Result:** `fetch_url`
```




  

<!DOCTYPE html>
<html
  lang="en"
  
  data-color-mode="auto" data-light-theme="light" data-dark-theme="dark"
  data-a11y-animated-images="system" data-a11y-link-underlines="true"
  
  >




  <head>
    <meta charset="utf-8">
  <link rel="dns-prefetch" href="https://github.githubassets.com">
  <link rel="dns-prefetch" href="https://avatars.githubusercontent.com">
  <link rel="dns-prefetch" href="https://github-cloud.s3.amazonaws.com">
  <link rel="dns-prefetch" href="https://user-images.githubusercontent.com/">
  <link rel="preconnect" href="https://github.githubassets.com" crossorigin>
  <link rel="preconnect" href="https://avatars.githubusercontent.com">

<script type="importmap">{"imports":{"react":"https://github.githubassets.com/assets/react-e27d1b3e03961e68.js","react-dom":"https://github.githubassets.com/assets/react-dom-e5fd46a22d5c4058.js","react-dom/client":"https://github.githubassets.com/assets/react-dom-client-1b4a3ee065998cea.js","react-is":"https://github.githubassets.com/assets/react-is-e0b593954b4706d8.js","react-reconciler":"https://github.githubassets.com/assets/react-reconciler-8e99e505c4429605.js","react/compiler-runtime":"https://github.githubassets.com/assets/react-compiler-runtime-4610bd6d3de9c049.js","react/jsx-dev-runtime":"https://github.githubassets.com/assets/react-jsx-dev-runtime-ea55d68667d559e5.js","react/jsx-runtime":"https://github.githubassets.com/assets/react-jsx-runtime-4915cb0f5b3aff04.js","scheduler":"https://github.githubassets.com/assets/scheduler-58b860b049ca307c.js"}}</script>
<meta name="react-profiling" content="0" data-turbo-transient="true" />
<meta name="react-import-map" content="react,react-dom,react-dom/client,react-dom/profiling,react-is,react-reconciler,react/compiler-runtime,react/jsx-dev-runtime,react/jsx-runtime,scheduler@777f63f87cd0" data-turbo-track="reload" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-e27d1b3e03961e68.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-compiler-runtime-4610bd6d3de9c049.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/scheduler-58b860b049ca307c.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-dom-e5fd46a22d5c4058.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-dom-client-1b4a3ee065998cea.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-is-e0b593954b4706d8.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-jsx-runtime-4915cb0f5b3aff04.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-reconciler-8e99e505c4429605.js" />

  


  <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/light-99f877e9ddfc0e51.css" /><link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/light_high_contrast-48fdd0811afbab3c.css" /><link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/dark-79ad2ace604703b3.css" /><link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/dark_high_contrast-24484a076f02295f.css" /><link data-color-theme="light" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light-99f877e9ddfc0e51.css" /><link data-color-theme="light_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light_high_contrast-48fdd0811afbab3c.css" /><link data-color-theme="light_colorblind" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light_colorblind-f4bf1142976e4bbf.css" /><link data-color-theme="light_colorblind_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light_colorblind_high_contrast-f661b49995ba0bd8.css" /><link data-color-theme="light_tritanopia" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light_tritanopia-0b38d22346321c92.css" /><link data-color-theme="light_tritanopia_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light_tritanopia_high_contrast-f1c62c9e70259b9f.css" /><link data-color-theme="dark" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark-79ad2ace604703b3.css" /><link data-color-theme="dark_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_high_contrast-24484a076f02295f.css" /><link data-color-theme="dark_colorblind" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_colorblind-f50cacf0a86b9929.css" /><link data-color-theme="dark_colorblind_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_colorblind_high_contrast-e61d4f4ca17852c2.css" /><link data-color-theme="dark_tritanopia" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_tritanopia-39c10993d5603fac.css" /><link data-color-theme="dark_tritanopia_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_tritanopia_high_contrast-73236c840c0c7d90.css" /><link data-color-theme="dark_dimmed" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_dimmed-0de76f07cc035b10.css" /><link data-color-theme="dark_dimmed_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_dimmed_high_contrast-fd1500c8744e40d6.css" />

  <style type="text/css">
    :root {
      --tab-size-preference: 4;
    }

    pre, code {
      tab-size: var(--tab-size-preference);
    }
  </style>

    <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-primitives-ed9ca172356fd545.css" />
    <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-1d2c7f7b52a6068b.css" />
    <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/global-54ba76e934a49d7c.css" />
    <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/github-456bd5deb85c7ecd.css" />
  <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/repository-11ee8a031c040c1a.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/code-2d56bdb0166c0238.css" />

  

  <script type="application/json" id="client-env">{"locale":"en","featureFlags":["actions_enable_background_steps","actions_new_hosted_runner_image_select_sizes_and_versions","activity_diff_file_tree","activity_repos_file_tree","activity_repos_overview_header","activity_repos_overview_sidebar","agent_author_search_expansion","agent_author_search_expansion_ui_pulls","alternate_user_config_repo","async_conversion_coverage_enabled","billing_billable_licenses_cost_center_bucket_fix","billing_budget_expiration","billing_cost_center_list_assigned_resources","billing_discount_threshold_notification","code_quality_enablement_banner_targeting","code_quality_remove_preview","code_view_raf_sticky_lines","codespaces_prebuild_region_target_update","coding_agent_third_party_model_ui","copilot_agent_snippy","copilot_api_agentic_issue_marshal_yaml","copilot_automation_repo_mcp_servers","copilot_automations_pagination","copilot_chat_auto_mode_v2","copilot_chat_clear_model_selection_for_default_change","copilot_chat_vision_dotcom_chat_ga_gate","copilot_css_textarea_autosize","copilot_custom_copilots","copilot_custom_copilots_feature_preview","copilot_duplicate_thread","copilot_extensions_removal_on_marketplace","copilot_fix_failed_workflows_all_skus","copilot_hide_hovercard","copilot_immersive_task_hyperlinking","copilot_mc_cli_resume_any_users_task","copilot_mission_control_agent_merge_fix_ci","copilot_mission_control_agent_merge_resolve_conflicts","copilot_mission_control_early_stop","copilot_mission_control_environment_list_icons","copilot_mission_control_managed_sandbox_environments","copilot_mission_control_needs_attention","copilot_mission_control_reasoning_effort","copilot_mission_control_sandbox_client_side_clone","copilot_mission_control_sandbox_remote_bypass","copilot_mission_control_session_filters","copilot_mission_control_task_alive_updates","copilot_mission_control_task_sharing","copilot_org_policy_page_focus_mode","copilot_share_active_subthread","copilot_spaces_ga","copilot_spaces_individual_policies_ga","copilot_swe_agent_authorization_status_ui","copilot_swe_agent_automation_resource_scoped_writes","copilot_swe_agent_hide_model_picker_if_only_auto","copilot_swe_agent_issue_comment_trigger","copilot_swe_agent_pr_comment_model_picker","copilot_swe_agent_pull_request_comment_trigger","copilot_swe_agent_pull_request_merged_trigger","copilot_swe_agent_pull_request_opened_trigger","copilot_swe_agent_pull_request_synchronize_trigger","copilot_swe_agent_use_subagents","copilot_task_api_github_rest_style","copilot_task_scoped_alive_channel","copilot_token_based_billing","copilot_unconfigured_is_inherited","copilot_user_can_upgrade_plan_field","copilot_workbench_sunset_redirect","dashboard_indexeddb_caching","dashboard_lists_max_age_filter","dashboard_universe_2025_feedback_dialog","fgpat_permissions_selector_redesign","glc_code_quality_repo_settings_workflow_config","hide_github_models_ui","hide_groups_list_for_few_groups","hyperspace_2025_logged_out_batch_1","hyperspace_2025_logged_out_batch_2","hyperspace_2025_logged_out_batch_3","in_product_messaging_datadog_monitoring","ipm_ubb_individual_budget_banner","issue_fields_multi_select","issue_inline_avatars","issue_pinned_views","issue_relative_time_micro","issues_dashboard_sso_structured_errors","issues_expanded_file_types","issues_hide_closed_sub_issues","issues_lazy_load_comment_box_suggestions","issues_react_chrome_container_query_fix","issues_relates_to_projects_search","labels_archiving","labels_archiving_info","landing_pages_ninetailed","lifecycle_label_name_updates","marketing_cookie_consent_banner","marketing_pages_search_explore_provider","memex_default_issue_create_repository","memex_lazy_hydrate_agent_tasks","memex_live_update_hovercard","memex_mwl_filter_field_delimiter","memex_remove_deprecated_type_issue","merge_queue_restricted_pushers_warning","merge_status_checks_refetch_dedupe","merge_status_header_feedback","mergeability_refetch_dedupe","octocaptcha_origin_optimization","primer_react_css_anchor_positioning","primer_react_merged_forwarded_refs","prs_copilot_app_open_action","prs_css_anchor_positioning","prs_thread_reply_as_primary_action","pull_request_copilot_attribution_header","pull_request_overview_panel_edit_description","pull_request_persister","pull_request_stacks_feedback_dialog","pull_request_virtualization_image_estimate","pull_request_virtualization_loader_batching","pull_request_virtualization_scroll_compensation","pull_request_virtualization_scroll_intent","quick_search_lazy_suggestions","react_blob_isolate_code_lines","react_blob_ssr_content_visibility","react_data_router_tanstack_allowed","react_query_props_with_key","react_sandbox_future_tanstack","repo_app_turbo","repo_issues_sidebar_layout","repo_pulls_dashboard_declutter","repo_pulls_dashboard_ga","repo_pulls_dashboard_persistence","repo_pulls_dashboard_sidebar_links","repos_contributors_limited_default_range","review_involves_filter","rule_ignored_file_paths","rulesets_actor_list_editor","sample_network_conn_type","sanitize_preset_flash_error","sanitize_preset_validation_message","security_center_artifact_filters_popover","see_who_reacted","semantic_similarity_duplicate_issue_detection","session_logs_ungroup_reasoning_text","set_sha256_on_repo_creation_form","site_banner_desktop_copilot_app","site_ghca_pixel_mona","site_github_app_ga_page","site_github_app_ga_page_highlight","site_github_app_mobile_native_share","site_global_banner_dev_days_attendee","site_global_banner_learn_copilot_sdk","site_global_nav_spark_models_removed","speculation_rules_ui_service","suggest_custom_property_values_copilot","suppress_automated_browser_vitals","swp_forms_disable_octocaptcha","thread_resolution_reason","ui_service_referrer_metrics","update_issue_suggestions","user_code_paste_ux","viewscreen_sandbox","warn_inaccessible_attachments","webp_support","workstream_plugin_bootstrap"],"githubDomain":"https://github.com","copilotApiOverrideUrl":"https://api.githubcopilot.com","cmcApiUrl":"https://api.github.com/cmc_internal/api"}</script>
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/high-contrast-cookie-e3d808ee18eb9784.js"></script>
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/wp-runtime-05d1942afe214994.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/app-foundation-74e9766bd696d26a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/app-runtime-7d377d1f27bbad23.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/fetch-utilities-9a5c31efd5313aa8.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ser-cd1b421ad0ad7ed5.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/environment-b3a091d0025aba04.js" defer="defer"></script>
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/app-runtime.9246aa20d02bb90b.module.css" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/catalyst-044fa3a2acdaf0a9.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/selector-observer-e88088f989b27670.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/relative-time-element-7cdf4e0db98b997c.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/by-6b3c07383371bf58.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ja9-36f036c13c2e3a4c.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/kw-866fd9513ae95106.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/r1-b075aab3204f32f9.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/jz5-7c5d1669717041f8.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/hj-5126390ff433704d.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/j05-dce4d588268f0f7e.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/github-elements-c45a71167e34440f.js" defer="defer"></script>
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/element-registry-db7d32eddf19b9b9.js" defer="defer"></script>
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/runtime-helpers-6df03a157bacbc44.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/aria-live-76b18916d0c8ec4d.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/hotkey-5109c77d7ac61078.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-core-d27b938e851ccb49.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/04-b4570e9cc17d8b62.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ur-024971cb63acad59.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/4t-364e19c63d726e9a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/6ny-c969fbae697a356f.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/yhq-3a209035a30b1a09.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/z1v-da5b33b877bfa358.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/wo-df202594a5bf5f61.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/9y-ebc4195ed57c6001.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/j0-41a4cf04cbd99503.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/0p-81491ed0d9d666e2.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/behaviors-e280187a9767d3e6.js" defer="defer"></script>
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/react-core.5c6ee197c091b384.module.css" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/n9-97e2e5539969aaaf.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/codespaces-a427ff8c1198f956.js" defer="defer"></script>
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ic-991f04433f366e7b.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/repositories-b85d65c9fda8a709.js" defer="defer"></script>
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/code-menu-d49e4b9c111b9b3e.js" defer="defer"></script>
  
  <link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/primer-react-3ca70c6e707ba409.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ncx-9a860680a5f230b9.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/j6-363f0ebf4f62f310.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/t7l-bf6f21d7e7a43c35.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/4u-27f74cfc703e1977.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/wsr-5fd3071e5a9061ab.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/0n-ecadb2ffe6b01ba2.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/s07-3fae157786a7acb1.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/n4u-6317ce1c31aed1db.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/j8s-d90ce3a169b741ed.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/la-d2cd96f070e6b52a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/q61-80cd2b936a4f27b6.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/fj8-b6b4dd5c8ee28797.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/w4q-134941577ca76346.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/qhu-cfdd732196b44b49.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/gzu-b9f61517dfb069e5.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/bi0-a78ba744687848c2.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/on-c7e4c90af6ffd527.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/b1-04bc5715b6dd3e67.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/0l5-2c7e8df6e59adc66.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/c2z-c92998400310b94b.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/f9-038758dcbfe08dab.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/0k-b7df0a74912a42bf.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/sp-352b18edf58f7022.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/og-72a377e4a14147a1.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/b9-4fd36ab02c30bf52.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ec-cbba6ef3015afe55.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/du-3326f9d530cbf97a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/20-e56a5cb6a2f13e42.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ql0-e077eae00b3c661b.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/dsj-cb1e4ddbce4f007b.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/wp-9f5235d335ab624b.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/gc4-c5927d67cdc592a2.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/uw-1408814835e50727.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/7cw-169ae7bb5f04a269.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/z-831ad54ea056549a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/hrn-71934060c9ba1e3a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/w4-a0045409d63046ff.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/jxi-4a01b1e0ce757fd4.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/72-20143cc69e68a0e4.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/xc-f5d51b571cd317cb.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/t-7c4bc4aab3fcad0a.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/code-view-6763778c37f1fa1a.js" defer="defer"></script>
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-css.44b7afb50b7d2b27.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/ql0.a8799830227aa47a.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/z.79ba33c2ec5d04d0.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/xc.13e027137438fd9f.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/t.84009134db3f904a.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/code-view.a18f81c678089bdc.module.css" />

  <link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/8p0n-73376bed45f677a1.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/notifications-subscriptions-menu-77a1e558ed3fc653.js" defer="defer"></script>
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-css.44b7afb50b7d2b27.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/notifications-subscriptions-menu.b5fa61d23b9a02b8.module.css" />


  <title>GitHub - walley/certik: certik bertik · GitHub</title>



  <meta name="route-pattern" content="/:user_id/:repository" data-turbo-transient>
  <meta name="route-controller" content="files" data-turbo-transient>
  <meta name="route-action" content="disambiguate" data-turbo-transient>
  <meta name="fetch-nonce" content="v2:d44f9894-77ed-50bd-38db-c87f07fb85f0">

    
  <meta name="current-catalog-service-hash" content="f3abb0cc802f3d7b95fc8762b94bdcb13bf39634c40c357301c4aa1d67a256fb">


  <meta name="request-id" content="2712:316EC5:3DD2F53:3925846:6AB4F502" data-pjax-transient="true"/><meta name="html-safe-nonce" content="9355b6bf4f00a24dbc7e742b6f90a29bf3ad37880a13b0036d54f69cf195afa8" data-pjax-transient="true"/><meta name="visitor-payload" content="eyJyZWZlcnJlciI6IiIsInJlcXVlc3RfaWQiOiIyNzEyOjMxNkVDNTozREQyRjUzOjM5MjU4NDY6NkFCNEY1MDIiLCJ2aXNpdG9yX2lkIjoiODU5ODIwNzY1NDI4NTA3OTgxMCIsInJlZ2lvbl9lZGdlIjoiZnJhIiwicmVnaW9uX3JlbmRlciI6ImZyYSJ9" data-pjax-transient="true"/><meta name="visitor-hmac" content="203674c59d421d55c6c07fdded5d98aa61f9b5ea6d71fa2ffe67be9ad73cc942" data-pjax-transient="true"/>


    <meta name="hovercard-subject-tag" content="repository:1352076649" data-turbo-transient>


  <meta name="github-keyboard-shortcuts" content="repository,copilot" data-turbo-transient="true" />
  

  <meta name="selected-link" value="repo_source" data-turbo-transient>
  <link rel="assets" href="https://github.githubassets.com/">

    <meta name="google-site-verification" content="Apib7-x98H0j5cPqHWwSMm6dNU4GmODRoqxLiDzdx9I">

<meta name="octolytics-url" content="https://collector.github.com/github/collect" />





  <meta name="analytics-location" content="/&lt;user-name&gt;/&lt;repo-name&gt;" data-turbo-transient="true" />

  




    <meta name="user-login" content="">

  

    <meta name="viewport" content="width=device-width">

    

      <meta name="description" content="certik bertik. Contribute to walley/certik development by creating an account on GitHub.">

      <link rel="search" type="application/opensearchdescription+xml" href="/opensearch.xml" title="GitHub">

    <link rel="fluid-icon" href="https://github.com/fluidicon.png" title="GitHub">
    <meta property="fb:app_id" content="1401488693436528">
    <meta name="apple-itunes-app" content="app-id=1477376905, app-argument=https://github.com/walley/certik" />

      <meta name="twitter:image" content="https://opengraph.githubassets.com/5e048d1cd52f92ef2afe50b0ca8ab9f9303cdf6adf3fcf710173bc05de59129a/walley/certik" /><meta name="twitter:site" content="@github" /><meta name="twitter:card" content="summary_large_image" /><meta name="twitter:title" content="GitHub - walley/certik: certik bertik" /><meta name="twitter:description" content="certik bertik. Contribute to walley/certik development by creating an account on GitHub." />
  <meta property="og:image" content="https://opengraph.githubassets.com/5e048d1cd52f92ef2afe50b0ca8ab9f9303cdf6adf3fcf710173bc05de59129a/walley/certik" /><meta property="og:image:alt" content="certik bertik. Contribute to walley/certik development by creating an account on GitHub." /><meta property="og:image:width" content="1200" /><meta property="og:image:height" content="600" /><meta property="og:site_name" content="GitHub" /><meta property="og:type" content="object" /><meta property="og:title" content="GitHub - walley/certik: certik bertik" /><meta property="og:url" content="https://github.com/walley/certik" /><meta property="og:description" content="certik bertik. Contribute to walley/certik development by creating an account on GitHub." />
  




      <meta name="hostname" content="github.com">



        <meta name="expected-hostname" content="github.com">


  <meta http-equiv="x-pjax-version" content="e8147740551c17ad4649779964afcf0f7c6e6fbf2d69fcf6f95e5def83e9b343" data-turbo-track="reload">
  <meta http-equiv="x-pjax-csp-version" content="c4a65e47b0c850e1157ae8e66298070851d7e3b558038de5a3b4ff7a93e630ed" data-turbo-track="reload">
  <meta http-equiv="x-pjax-css-version" content="ec4c7b86d7b57240253150b827ea48b04e08d8760dc6553d2a04f62000f3fe0c" data-turbo-track="reload">
  <meta http-equiv="x-pjax-js-version" content="982f0a2c1f3dcde233dc214868b7e87d18cebdb6b2c83b98a936c8f73474bba4" data-turbo-track="reload">

  <meta name="turbo-cache-control" content="no-preview" data-turbo-transient="">

      <meta name="turbo-cache-control" content="no-cache" data-turbo-transient>

    <meta data-hydrostats="publish">

  <meta name="go-import" content="github.com/walley/certik git https://github.com/walley/certik.git">

  <meta name="octolytics-dimension-user_id" content="236163" /><meta name="octolytics-dimension-user_login" content="walley" /><meta name="octolytics-dimension-repository_id" content="1352076649" /><meta name="octolytics-dimension-repository_nwo" content="walley/certik" /><meta name="octolytics-dimension-repository_public" content="true" /><meta name="octolytics-dimension-repository_is_fork" content="false" /><meta name="octolytics-dimension-repository_network_root_id" content="1352076649" /><meta name="octolytics-dimension-repository_network_root_nwo" content="walley/certik" />
  



    

    <meta name="turbo-body-classes" content="logged-out env-production page-responsive">
  <meta name="disable-turbo" content="false">


  <meta name="browser-stats-url" content="https://api.github.com/_private/browser/stats">


  <meta name="browser-errors-url" content="https://api.github.com/_private/browser/errors">

  <meta name="release" content="30be1065334e97b0e7c320e73f0a6289d47eed36" data-turbo-track="reload">
  <meta name="ui-target" content="full">

  <link rel="mask-icon" href="https://github.githubassets.com/assets/pinned-octocat-093da3e6fa40.svg" color="#000000">
  <link rel="alternate icon" class="js-site-favicon" type="image/png" href="https://github.githubassets.com/favicons/favicon.png">
  <link rel="icon" class="js-site-favicon" type="image/svg+xml" href="https://github.githubassets.com/favicons/favicon.svg" data-base-href="https://github.githubassets.com/favicons/favicon">

<meta name="theme-color" content="#1e2327">
<meta name="color-scheme" content="light dark" />


  <link rel="manifest" href="/manifest.json" crossOrigin="use-credentials">

  </head>

  <body class="logged-out env-production page-responsive" style="word-wrap: break-word;" >
    <div data-turbo-body class="logged-out env-production page-responsive" style="word-wrap: break-word;" >
      <div id="__primerPortalRoot__" style="z-index: 1000; position: absolute; width: 100%;" data-turbo-permanent></div>
      

    <div class="position-relative header-wrapper js-header-wrapper ">
      <a href="#start-of-content" data-skip-target-assigned="false" class="px-2 tmp-py-4 color-bg-accent-emphasis color-fg-on-emphasis show-on-focus js-skip-to-content">Skip to content</a>

      <span data-view-component="true" class="progress-pjax-loader Progress position-fixed width-full">
    <span style="width: 0%;" data-view-component="true" class="Progress-item progress-pjax-loader-bar left-0 top-0 color-bg-accent-emphasis"></span>
</span>      
      <link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/l7j-2ddd8b435dadae28.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/keyboard-shortcuts-dialog-fa6bdebc4a2ffedb.js" fetchpriority="low" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-css.44b7afb50b7d2b27.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/keyboard-shortcuts-dialog.4f138c293c5b3011.module.css" />

<react-partial
  partial-name="keyboard-shortcuts-dialog"
  data-ssr="false"
  data-attempted-ssr="false"
  data-react-profiling="false"
>
  
  <script type="application/json" data-target="react-partial.embeddedData">{"props":{"docsUrl":"https://docs.github.com/get-started/accessibility/keyboard-shortcuts"}}</script>
  <div data-target="react-partial.reactRoot"></div>
</react-partial>





      

          <link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/pw-4f91498888595802.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/app-install-banner-partial-8dfb3ab112a86218.js" fetchpriority="low" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-css.44b7afb50b7d2b27.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/app-install-banner-partial.2b3a47cb3223afb5.module.css" />

<react-partial
  partial-name="app-install-banner-partial"
  data-ssr="false"
  data-attempted-ssr="false"
  data-react-profiling="false"
>
  
  <script type="application/json" data-target="react-partial.embeddedData">{"props":{}}</script>
  <div data-target="react-partial.reactRoot"></div>
</react-partial>


          

                <link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/2l-6cedbc3128429fc7.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/i2-3e1ea9a828b29b0d.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/fz5-23f28c0f388769c9.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/lht-dedd9949bed91081.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/p1-76a75a6081059b19.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/3d-82c347d20a13fb90.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/tua-18c20c09a9dc80b9.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/t1-7addb7a44e70fa3c.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/marketing-header-204eb35de84822f3.js" fetchpriority="low" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-css.44b7afb50b7d2b27.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-brand-css.6a01c236ff527e7e.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/t1.a304baa4c3062c45.module.css" />

<react-partial
  partial-name="marketing-header"
  data-ssr="true"
  data-attempted-ssr="true"
  data-react-profiling="false"
>
  
  <script type="application/json" data-target="react-partial.embeddedData">{"props":{"color_mode":"dark","logged_in":false,"marketing_page":false,"home_path":"/","login_path":"/login?return_to=https%3A%2F%2Fgithub.com%2Fwalley%2Fcertik","signup_path":"/signup?ref_cta=Sign+up\u0026ref_loc=header+logged+out\u0026ref_page=%2F%3Cuser-name%3E%2F%3Crepo-name%3E\u0026source=header-repo\u0026source_repo=walley%2Fcertik","signup_enabled":true,"is_signup_controller":false,"show_search_and_nav":true,"hide_search":false,"private_mode_enabled":false,"should_use_dotcom_links":true,"auth_hydro_click":"{\"event_type\":\"authentication.click\",\"payload\":{\"location_in_page\":\"site header menu\",\"repository_id\":null,\"auth_type\":\"SIGN_UP\",\"originating_url\":\"https://github.com/walley/certik\",\"user_id\":null}}","auth_hydro_click_hmac":"e0db1f9357770cf412f50c21eefbca5d827a57b4f8aabce5e530a31131bdf8dc","overlay":false,"fixed":false}}</script>
  <div data-target="react-partial.reactRoot"><div data-color-mode="dark" data-light-theme="light" data-dark-theme="dark"><header class="MarketingHeader-module__root__Tk7n3 HeaderMktg header-logged-out" role="banner" data-marketing-header="true" data-color-mode="dark" data-light-theme="light" data-dark-theme="dark" data-is-top="true"><h2 class="MarketingHeader-module__visuallyHidden__sqKsl">Navigation Menu</h2><button type="button" class="MarketingHeader-module__backdrop__sw4RU" aria-label="Close navigation menu"></button><div class="MarketingHeader-module__bar__mBSyE"><div class="MarketingHeader-module__topRow__yeury"><div class="MarketingHeader-module__toggleSlot__hDxbh"><button type="button" class="HeaderMenuToggle-module__toggle__i8EiC" aria-label="Toggle navigation" aria-expanded="false"><span class="HeaderMenuToggle-module__toggleBar__jVN0H"></span><span class="HeaderMenuToggle-module__toggleBar__jVN0H"></span><span class="HeaderMenuToggle-module__toggleBar__jVN0H"></span></button></div><a href="/" aria-label="Homepage" class="HeaderLogo-module__logo__UFyHI" data-analytics-event="{&quot;action&quot;:&quot;homepage&quot;,&quot;tag&quot;:&quot;link&quot;,&quot;context&quot;:&quot;logo&quot;,&quot;location&quot;:&quot;header&quot;,&quot;label&quot;:&quot;homepage_link_logo_header&quot;}"><svg data-component="Octicon" aria-hidden="true" focusable="false" class="octicon octicon-mark-github" viewBox="0 0 24 24" width="32" height="32" fill="currentColor" display="inline-block" overflow="visible" style="vertical-align:text-bottom"><path d="M10.226 17.284c-2.965-.36-5.054-2.493-5.054-5.256 0-1.123.404-2.336 1.078-3.144-.292-.741-.247-2.314.09-2.965.898-.112 2.111.36 2.83 1.01.853-.269 1.752-.404 2.853-.404 1.1 0 1.999.135 2.807.382.696-.629 1.932-1.1 2.83-.988.315.606.36 2.179.067 2.942.72.854 1.101 2 1.101 3.167 0 2.763-2.089 4.852-5.098 5.234.763.494 1.28 1.572 1.28 2.807v2.336c0 .674.561 1.056 1.235.786 4.066-1.55 7.255-5.615 7.255-10.646C23.5 6.188 18.334 1 11.978 1 5.62 1 .5 6.188.5 12.545c0 4.986 3.167 9.12 7.435 10.669.606.225 1.19-.18 1.19-.786V20.63a2.9 2.9 0 0 1-1.078.224c-1.483 0-2.359-.808-2.987-2.313-.247-.607-.517-.966-1.034-1.033-.27-.023-.359-.135-.359-.27 0-.27.45-.471.898-.471.652 0 1.213.404 1.797 1.235.45.651.921.943 1.483.943.561 0 .92-.202 1.437-.719.382-.381.674-.718.944-.943"></path></svg></a><div class="AuthCTAs-module__mobileActions__NNzeV"><a class="Primer_Brand__Button-module__Button___scH9Z Primer_Brand__Button-module__Button--subtle___F7pEE Primer_Brand__Button-module__Button--size-small___zQrEw AuthCTAs-module__cta__WpwQq" href="/login?return_to=https%3A%2F%2Fgithub.com%2Fwalley%2Fcertik" data-analytics-event="{&quot;action&quot;:&quot;sign_in&quot;,&quot;tag&quot;:&quot;link&quot;,&quot;context&quot;:&quot;auth_cta&quot;,&quot;location&quot;:&quot;header&quot;,&quot;label&quot;:&quot;sign_in_link_auth_cta_header&quot;}" data-hydro-click="{&quot;event_type&quot;:&quot;authentication.click&quot;,&quot;payload&quot;:{&quot;location_in_page&quot;:&quot;site header menu&quot;,&quot;repository_id&quot;:null,&quot;auth_type&quot;:&quot;SIGN_UP&quot;,&quot;originating_url&quot;:&quot;https://github.com/walley/certik&quot;,&quot;user_id&quot;:null}}" data-hydro-click-hmac="e0db1f9357770cf412f50c21eefbca5d827a57b4f8aabce5e530a31131bdf8dc"><span class="Primer_Brand__Button-module__Button__text___ED0bX"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--100___B2ueX Primer_Brand__Text-module__Text--weight-medium___qJKf_ Primer_Brand__Button-module__Button--label___qrkyz Primer_Brand__Button-module__Button--label-subtle___8ndWH">Sign in</span></span></a><button class="Primer_Brand__Button-module__Button___scH9Z Primer_Brand__Button-module__Button--subtle___F7pEE Primer_Brand__Button-module__Button--size-small___zQrEw HeaderAppearanceSettings-module__trigger__hUheK" type="button" aria-haspopup="dialog" aria-labelledby="_R_3dd_"><span class="Primer_Brand__Button-module__Button__leading-visual___jjtTe" data-testid="Button-leading-visual"><svg data-component="Octicon" focusable="false" aria-hidden="true" class="octicon octicon-sliders Primer_Brand__Button-module__Button__icon-visual____qybb" viewBox="0 0 16 16" width="16" height="16" fill="currentColor" display="inline-block" overflow="visible" style="vertical-align:text-bottom"><path d="M15 2.75a.75.75 0 0 1-.75.75h-4a.75.75 0 0 1 0-1.5h4a.75.75 0 0 1 .75.75Zm-8.5.75v1.25a.75.75 0 0 0 1.5 0v-4a.75.75 0 0 0-1.5 0V2H1.75a.75.75 0 0 0 0 1.5H6.5Zm1.25 5.25a.75.75 0 0 0 0-1.5h-6a.75.75 0 0 0 0 1.5h6ZM15 8a.75.75 0 0 1-.75.75H11.5V10a.75.75 0 1 1-1.5 0V6a.75.75 0 0 1 1.5 0v1.25h2.75A.75.75 0 0 1 15 8Zm-9 5.25v-2a.75.75 0 0 0-1.5 0v1.25H1.75a.75.75 0 0 0 0 1.5H4.5v1.25a.75.75 0 0 0 1.5 0v-2Zm9 0a.75.75 0 0 1-.75.75h-6a.75.75 0 0 1 0-1.5h6a.75.75 0 0 1 .75.75Z"></path></svg></span><span class="Primer_Brand__Button-module__Button__text___ED0bX"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--100___B2ueX Primer_Brand__Text-module__Text--weight-medium___qJKf_ Primer_Brand__Button-module__Button--label___qrkyz Primer_Brand__Button-module__Button--label-subtle___8ndWH"></span></span></button><div class="Primer_Brand__Tooltip-module__Tooltip___0Eipx" data-direction="s" aria-hidden="true" id="_R_3dd_">Appearance settings</div></div></div><div class="MarketingHeader-module__menu__GIy3y"><div class="MarketingHeader-module__menuWrapper__owstH"><nav class="MarketingNavigation-module__nav__W0KYY" aria-label="Global"><ul class="MarketingNavigation-module__list__tFbMb"><li><div class="NavDropdown-module__container__l2YeI"><button type="button" class="NavDropdown-module__button__PEHWX" aria-expanded="false" aria-controls="_R_nd_">Platform<svg data-component="Octicon" aria-hidden="true" focusable="false" class="octicon octicon-triangle-right NavDropdown-module__buttonIcon__Tkl8_" viewBox="0 0 16 16" width="16" height="16" fill="currentColor" display="inline-block" overflow="visible" style="vertical-align:text-bottom"><path d="m6.427 4.427 3.396 3.396a.25.25 0 0 1 0 .354l-3.396 3.396A.25.25 0 0 1 6 11.396V4.604a.25.25 0 0 1 .427-.177Z"></path></svg></button><div id="_R_nd_" class="NavDropdown-module__dropdown__xm1jd"><ul class="NavDropdown-module__list__zuCgG"><li><div class="NavGroup-module__group__W8SqJ"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--monospace___QXHDQ Primer_Brand__Text-module__Text--muted___rE6mh Primer_Brand__Text-module__Text--100___B2ueX Primer_Brand__Text-module__Text--weight-medium___qJKf_ NavGroup-module__title__Wzxz2" id="_R_5knd_">AI CODE CREATION</span><ul class="NavGroup-module__list__UCOFy" aria-labelledby="_R_5knd_"><li><a class="Primer_Brand__Link-module__Link___lF11y Primer_Brand__Link-module__Link--default___VRVW0 NavLink-module__link__EG3d4" href="https://github.com/features/copilot" data-analytics-event="{&quot;action&quot;:&quot;github_copilot&quot;,&quot;tag&quot;:&quot;link&quot;,&quot;context&quot;:&quot;platform&quot;,&quot;location&quot;:&quot;navbar&quot;,&quot;label&quot;:&quot;github_copilot_link_platform_navbar&quot;}"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS Primer_Brand__Link-module__Link--label___jM8Ty"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS Primer_Brand__Text-module__Text--weight-medium___qJKf_ NavLink-module__title__Q7t0p"><svg data-component="Octicon" aria-hidden="true" focusable="false" class="octicon octicon-copilot NavLink-module__icon__ltGNM" viewBox="0 0 16 16" width="16" height="16" fill="currentColor" display="inline-block" overflow="visible" style="vertical-align:text-bottom"><path d="M7.998 15.035c-4.562 0-7.873-2.914-7.998-3.749V9.338c.085-.628.677-1.686 1.588-2.065.013-.07.024-.143.036-.218.029-.183.06-.384.126-.612-.201-.508-.254-1.084-.254-1.656 0-.87.128-1.769.693-2.484.579-.733 1.494-1.124 2.724-1.261 1.206-.134 2.262.034 2.944.765.05.053.096.108.139.165.044-.057.094-.112.143-.165.682-.731 1.738-.899 2.944-.765 1.23.137 2.145.528 2.724 1.261.566.715.693 1.614.693 2.484 0 .572-.053 1.148-.254 1.656.066.228.098.429.126.612.012.076.024.148.037.218.924.385 1.522 1.471 1.591 2.095v1.872c0 .766-3.351 3.795-8.002 3.795Zm0-1.485c2.28 0 4.584-1.11 5.002-1.433V7.862l-.023-.116c-.49.21-1.075.291-1.727.291-1.146 0-2.059-.327-2.71-.991A3.222 3.222 0 0 1 8 6.303a3.24 3.24 0 0 1-.544.743c-.65.664-1.563.991-2.71.991-.652 0-1.236-.081-1.727-.291l-.023.116v4.255c.419.323 2.722 1.433 5.002 1.433ZM6.762 2.83c-.193-.206-.637-.413-1.682-.297-1.019.113-1.479.404-1.713.7-.247.312-.369.789-.369 1.554 0 .793.129 1.171.308 1.371.162.181.519.379 1.442.379.853 0 1.339-.235 1.638-.54.315-.322.527-.827.617-1.553.117-.935-.037-1.395-.241-1.614Zm4.155-.297c-1.044-.116-1.488.091-1.681.297-.204.219-.359.679-.242 1.614.091.726.303 1.231.618 1.553.299.305.784.54 1.638.54.922 0 1.28-.198 1.442-.379.179-.2.308-.578.308-1.371 0-.765-.123-1.242-.37-1.554-.233-.296-.693-.587-1.713-.7Z"></path><path d="M6.25 9.037a.75.75 0 0 1 .75.75v1.501a.75.75 0 0 1-1.5 0V9.787a.75.75 0 0 1 .75-.75Zm4.25.75v1.501a.75.75 0 0 1-1.5 0V9.787a.75.75 0 0 1 1.5 0Z"></path></svg>GitHub Copilot</span><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--muted___rE6mh Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS NavLink-module__subtitle__X4gkW">Write better code with AI</span></span></a></li><li><a class="Primer_Brand__Link-module__Link___lF11y Primer_Brand__Link-module__Link--default___VRVW0 NavLink-module__link__EG3d4" href="https://github.com/features/ai/github-app" data-analytics-event="{&quot;action&quot;:&quot;github_copilot_app&quot;,&quot;tag&quot;:&quot;link&quot;,&quot;context&quot;:&quot;platform&quot;,&quot;location&quot;:&quot;navbar&quot;,&quot;label&quot;:&quot;github_copilot_app_link_platform_navbar&quot;}"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS Primer_Brand__Link-module__Link--label___jM8Ty"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS Primer_Brand__Text-module__Text--weight-medium___qJKf_ NavLink-module__title__Q7t0p"><svg data-component="Octicon" aria-hidden="true" focusable="false" class="octicon octicon-mark-github NavLink-module__icon__ltGNM" viewBox="0 0 16 16" width="16" height="16" fill="currentColor" display="inline-block" overflow="visible" style="vertical-align:text-bottom"><path d="M6.766 11.328c-2.063-.25-3.516-1.734-3.516-3.656 0-.781.281-1.625.75-2.188-.203-.515-.172-1.609.063-2.062.625-.078 1.468.25 1.968.703.594-.187 1.219-.281 1.985-.281.765 0 1.39.094 1.953.265.484-.437 1.344-.765 1.969-.687.218.422.25 1.515.046 2.047.5.593.766 1.39.766 2.203 0 1.922-1.453 3.375-3.547 3.64.531.344.89 1.094.89 1.954v1.625c0 .468.391.734.86.547C13.781 14.359 16 11.53 16 8.03 16 3.61 12.406 0 7.984 0 3.563 0 0 3.61 0 8.031a7.88 7.88 0 0 0 5.172 7.422c.422.156.828-.125.828-.547v-1.25c-.219.094-.5.156-.75.156-1.031 0-1.64-.562-2.078-1.609-.172-.422-.36-.672-.719-.719-.187-.015-.25-.093-.25-.187 0-.188.313-.328.625-.328.453 0 .844.281 1.25.86.313.452.64.655 1.031.655s.641-.14 1-.5c.266-.265.47-.5.657-.656"></path></svg>GitHub Copilot app</span><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--muted___rE6mh Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS NavLink-module__subtitle__X4gkW">Direct agents from issue to merge</span></span></a></li><li><a class="Primer_Brand__Link-module__Link___lF11y Primer_Brand__...[truncated, total 245368 bytes]
```

_Thinking:_ The GitHub page fetched is the full HTML which is very large. I need to extract project information from it. Let me search for relevant meta tags or data that I can use. I notice there's a description meta tag: `<meta name="description" content="certik bertik. Contribute to walley/certik development by creating an account on GitHub.">`

Let me try to fetch the other repositories and also look for structured data. Let me search for specific patterns in the fetched content.

Actually, let me take a different approach. Since the GitHub API is blocked, I can try to extract information from the HTML pages I've fetched. Let me search for repository metadata in the fetched HTML.

Let me look at what info I can extract from the certik page. I see the description meta tag. Let me also try to fetch the other two repos.

Actually, let me think about this differently. I need to create an HTTP page displaying the user's projects with descriptions. Since I can't easily get the GitHub API data, I could:

1. Try to extract info from the HTML pages I've already fetched
2. Or create a simple HTML page with placeholder info that looks good

Let me try fetching the other two repositories to see if I can get similar metadata.

**Assistant:** Let me fetch the other two repositories to get their information.



**Tool Call:** `fetch_url`
```
{"url":"https://github.com/walley/bertik"}
```

**Tool Result:** `fetch_url`
```




  

<!DOCTYPE html>
<html
  lang="en"
  
  data-color-mode="auto" data-light-theme="light" data-dark-theme="dark"
  data-a11y-animated-images="system" data-a11y-link-underlines="true"
  
  >




  <head>
    <meta charset="utf-8">
  <link rel="dns-prefetch" href="https://github.githubassets.com">
  <link rel="dns-prefetch" href="https://avatars.githubusercontent.com">
  <link rel="dns-prefetch" href="https://github-cloud.s3.amazonaws.com">
  <link rel="dns-prefetch" href="https://user-images.githubusercontent.com/">
  <link rel="preconnect" href="https://github.githubassets.com" crossorigin>
  <link rel="preconnect" href="https://avatars.githubusercontent.com">

<script type="importmap">{"imports":{"react":"https://github.githubassets.com/assets/react-e27d1b3e03961e68.js","react-dom":"https://github.githubassets.com/assets/react-dom-e5fd46a22d5c4058.js","react-dom/client":"https://github.githubassets.com/assets/react-dom-client-1b4a3ee065998cea.js","react-is":"https://github.githubassets.com/assets/react-is-e0b593954b4706d8.js","react-reconciler":"https://github.githubassets.com/assets/react-reconciler-8e99e505c4429605.js","react/compiler-runtime":"https://github.githubassets.com/assets/react-compiler-runtime-4610bd6d3de9c049.js","react/jsx-dev-runtime":"https://github.githubassets.com/assets/react-jsx-dev-runtime-ea55d68667d559e5.js","react/jsx-runtime":"https://github.githubassets.com/assets/react-jsx-runtime-4915cb0f5b3aff04.js","scheduler":"https://github.githubassets.com/assets/scheduler-58b860b049ca307c.js"}}</script>
<meta name="react-profiling" content="0" data-turbo-transient="true" />
<meta name="react-import-map" content="react,react-dom,react-dom/client,react-dom/profiling,react-is,react-reconciler,react/compiler-runtime,react/jsx-dev-runtime,react/jsx-runtime,scheduler@777f63f87cd0" data-turbo-track="reload" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-e27d1b3e03961e68.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-compiler-runtime-4610bd6d3de9c049.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/scheduler-58b860b049ca307c.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-dom-e5fd46a22d5c4058.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-dom-client-1b4a3ee065998cea.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-is-e0b593954b4706d8.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-jsx-runtime-4915cb0f5b3aff04.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-reconciler-8e99e505c4429605.js" />

  


  <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/light-99f877e9ddfc0e51.css" /><link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/light_high_contrast-48fdd0811afbab3c.css" /><link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/dark-79ad2ace604703b3.css" /><link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/dark_high_contrast-24484a076f02295f.css" /><link data-color-theme="light" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light-99f877e9ddfc0e51.css" /><link data-color-theme="light_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light_high_contrast-48fdd0811afbab3c.css" /><link data-color-theme="light_colorblind" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light_colorblind-f4bf1142976e4bbf.css" /><link data-color-theme="light_colorblind_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light_colorblind_high_contrast-f661b49995ba0bd8.css" /><link data-color-theme="light_tritanopia" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light_tritanopia-0b38d22346321c92.css" /><link data-color-theme="light_tritanopia_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light_tritanopia_high_contrast-f1c62c9e70259b9f.css" /><link data-color-theme="dark" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark-79ad2ace604703b3.css" /><link data-color-theme="dark_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_high_contrast-24484a076f02295f.css" /><link data-color-theme="dark_colorblind" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_colorblind-f50cacf0a86b9929.css" /><link data-color-theme="dark_colorblind_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_colorblind_high_contrast-e61d4f4ca17852c2.css" /><link data-color-theme="dark_tritanopia" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_tritanopia-39c10993d5603fac.css" /><link data-color-theme="dark_tritanopia_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_tritanopia_high_contrast-73236c840c0c7d90.css" /><link data-color-theme="dark_dimmed" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_dimmed-0de76f07cc035b10.css" /><link data-color-theme="dark_dimmed_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_dimmed_high_contrast-fd1500c8744e40d6.css" />

  <style type="text/css">
    :root {
      --tab-size-preference: 4;
    }

    pre, code {
      tab-size: var(--tab-size-preference);
    }
  </style>

    <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-primitives-ed9ca172356fd545.css" />
    <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-1d2c7f7b52a6068b.css" />
    <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/global-54ba76e934a49d7c.css" />
    <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/github-456bd5deb85c7ecd.css" />
  <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/repository-11ee8a031c040c1a.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/code-2d56bdb0166c0238.css" />

  

  <script type="application/json" id="client-env">{"locale":"en","featureFlags":["actions_enable_background_steps","actions_new_hosted_runner_image_select_sizes_and_versions","activity_diff_file_tree","activity_repos_file_tree","activity_repos_overview_header","activity_repos_overview_sidebar","agent_author_search_expansion","agent_author_search_expansion_ui_pulls","alternate_user_config_repo","async_conversion_coverage_enabled","billing_billable_licenses_cost_center_bucket_fix","billing_budget_expiration","billing_cost_center_list_assigned_resources","billing_discount_threshold_notification","code_quality_enablement_banner_targeting","code_quality_remove_preview","code_view_raf_sticky_lines","codespaces_prebuild_region_target_update","coding_agent_third_party_model_ui","copilot_agent_snippy","copilot_api_agentic_issue_marshal_yaml","copilot_automation_repo_mcp_servers","copilot_automations_pagination","copilot_chat_auto_mode_v2","copilot_chat_clear_model_selection_for_default_change","copilot_chat_vision_dotcom_chat_ga_gate","copilot_css_textarea_autosize","copilot_custom_copilots","copilot_custom_copilots_feature_preview","copilot_duplicate_thread","copilot_extensions_removal_on_marketplace","copilot_fix_failed_workflows_all_skus","copilot_hide_hovercard","copilot_immersive_task_hyperlinking","copilot_mc_cli_resume_any_users_task","copilot_mission_control_agent_merge_fix_ci","copilot_mission_control_agent_merge_resolve_conflicts","copilot_mission_control_early_stop","copilot_mission_control_environment_list_icons","copilot_mission_control_managed_sandbox_environments","copilot_mission_control_needs_attention","copilot_mission_control_reasoning_effort","copilot_mission_control_sandbox_client_side_clone","copilot_mission_control_sandbox_remote_bypass","copilot_mission_control_session_filters","copilot_mission_control_task_alive_updates","copilot_mission_control_task_sharing","copilot_org_policy_page_focus_mode","copilot_share_active_subthread","copilot_spaces_ga","copilot_spaces_individual_policies_ga","copilot_swe_agent_authorization_status_ui","copilot_swe_agent_automation_resource_scoped_writes","copilot_swe_agent_hide_model_picker_if_only_auto","copilot_swe_agent_issue_comment_trigger","copilot_swe_agent_pr_comment_model_picker","copilot_swe_agent_pull_request_comment_trigger","copilot_swe_agent_pull_request_merged_trigger","copilot_swe_agent_pull_request_opened_trigger","copilot_swe_agent_pull_request_synchronize_trigger","copilot_swe_agent_use_subagents","copilot_task_api_github_rest_style","copilot_task_scoped_alive_channel","copilot_token_based_billing","copilot_unconfigured_is_inherited","copilot_user_can_upgrade_plan_field","copilot_workbench_sunset_redirect","dashboard_indexeddb_caching","dashboard_lists_max_age_filter","dashboard_universe_2025_feedback_dialog","fgpat_permissions_selector_redesign","glc_code_quality_repo_settings_workflow_config","hide_github_models_ui","hide_groups_list_for_few_groups","hyperspace_2025_logged_out_batch_1","hyperspace_2025_logged_out_batch_2","hyperspace_2025_logged_out_batch_3","in_product_messaging_datadog_monitoring","ipm_ubb_individual_budget_banner","issue_fields_multi_select","issue_inline_avatars","issue_pinned_views","issue_relative_time_micro","issues_dashboard_sso_structured_errors","issues_expanded_file_types","issues_hide_closed_sub_issues","issues_lazy_load_comment_box_suggestions","issues_react_chrome_container_query_fix","issues_relates_to_projects_search","labels_archiving","labels_archiving_info","landing_pages_ninetailed","lifecycle_label_name_updates","marketing_cookie_consent_banner","marketing_pages_search_explore_provider","memex_default_issue_create_repository","memex_lazy_hydrate_agent_tasks","memex_live_update_hovercard","memex_mwl_filter_field_delimiter","memex_remove_deprecated_type_issue","merge_queue_restricted_pushers_warning","merge_status_checks_refetch_dedupe","merge_status_header_feedback","mergeability_refetch_dedupe","octocaptcha_origin_optimization","primer_react_css_anchor_positioning","primer_react_merged_forwarded_refs","prs_copilot_app_open_action","prs_css_anchor_positioning","prs_thread_reply_as_primary_action","pull_request_copilot_attribution_header","pull_request_overview_panel_edit_description","pull_request_persister","pull_request_stacks_feedback_dialog","pull_request_virtualization_image_estimate","pull_request_virtualization_loader_batching","pull_request_virtualization_scroll_compensation","pull_request_virtualization_scroll_intent","quick_search_lazy_suggestions","react_blob_isolate_code_lines","react_blob_ssr_content_visibility","react_data_router_tanstack_allowed","react_query_props_with_key","react_sandbox_future_tanstack","repo_app_turbo","repo_issues_sidebar_layout","repo_pulls_dashboard_declutter","repo_pulls_dashboard_ga","repo_pulls_dashboard_persistence","repo_pulls_dashboard_sidebar_links","repos_contributors_limited_default_range","review_involves_filter","rule_ignored_file_paths","rulesets_actor_list_editor","sample_network_conn_type","sanitize_preset_flash_error","sanitize_preset_validation_message","security_center_artifact_filters_popover","see_who_reacted","semantic_similarity_duplicate_issue_detection","session_logs_ungroup_reasoning_text","set_sha256_on_repo_creation_form","site_banner_desktop_copilot_app","site_ghca_pixel_mona","site_github_app_ga_page","site_github_app_ga_page_highlight","site_github_app_mobile_native_share","site_global_banner_dev_days_attendee","site_global_banner_learn_copilot_sdk","site_global_nav_spark_models_removed","speculation_rules_ui_service","suggest_custom_property_values_copilot","suppress_automated_browser_vitals","swp_forms_disable_octocaptcha","thread_resolution_reason","ui_service_referrer_metrics","update_issue_suggestions","user_code_paste_ux","viewscreen_sandbox","warn_inaccessible_attachments","webp_support","workstream_plugin_bootstrap"],"githubDomain":"https://github.com","copilotApiOverrideUrl":"https://api.githubcopilot.com","cmcApiUrl":"https://api.github.com/cmc_internal/api"}</script>
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/high-contrast-cookie-e3d808ee18eb9784.js"></script>
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/wp-runtime-05d1942afe214994.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/app-foundation-74e9766bd696d26a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/app-runtime-7d377d1f27bbad23.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/fetch-utilities-9a5c31efd5313aa8.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ser-cd1b421ad0ad7ed5.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/environment-b3a091d0025aba04.js" defer="defer"></script>
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/app-runtime.9246aa20d02bb90b.module.css" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/catalyst-044fa3a2acdaf0a9.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/selector-observer-e88088f989b27670.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/relative-time-element-7cdf4e0db98b997c.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/by-6b3c07383371bf58.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ja9-36f036c13c2e3a4c.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/kw-866fd9513ae95106.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/r1-b075aab3204f32f9.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/jz5-7c5d1669717041f8.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/hj-5126390ff433704d.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/j05-dce4d588268f0f7e.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/github-elements-c45a71167e34440f.js" defer="defer"></script>
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/element-registry-db7d32eddf19b9b9.js" defer="defer"></script>
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/runtime-helpers-6df03a157bacbc44.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/aria-live-76b18916d0c8ec4d.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/hotkey-5109c77d7ac61078.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-core-d27b938e851ccb49.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/04-b4570e9cc17d8b62.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ur-024971cb63acad59.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/4t-364e19c63d726e9a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/6ny-c969fbae697a356f.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/yhq-3a209035a30b1a09.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/z1v-da5b33b877bfa358.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/wo-df202594a5bf5f61.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/9y-ebc4195ed57c6001.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/j0-41a4cf04cbd99503.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/0p-81491ed0d9d666e2.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/behaviors-e280187a9767d3e6.js" defer="defer"></script>
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/react-core.5c6ee197c091b384.module.css" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/n9-97e2e5539969aaaf.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/codespaces-a427ff8c1198f956.js" defer="defer"></script>
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ic-991f04433f366e7b.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/repositories-b85d65c9fda8a709.js" defer="defer"></script>
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/code-menu-d49e4b9c111b9b3e.js" defer="defer"></script>
  
  <link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/primer-react-3ca70c6e707ba409.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ncx-9a860680a5f230b9.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/j6-363f0ebf4f62f310.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/t7l-bf6f21d7e7a43c35.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/4u-27f74cfc703e1977.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/wsr-5fd3071e5a9061ab.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/0n-ecadb2ffe6b01ba2.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/s07-3fae157786a7acb1.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/n4u-6317ce1c31aed1db.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/j8s-d90ce3a169b741ed.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/la-d2cd96f070e6b52a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/q61-80cd2b936a4f27b6.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/fj8-b6b4dd5c8ee28797.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/w4q-134941577ca76346.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/qhu-cfdd732196b44b49.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/gzu-b9f61517dfb069e5.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/bi0-a78ba744687848c2.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/on-c7e4c90af6ffd527.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/b1-04bc5715b6dd3e67.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/0l5-2c7e8df6e59adc66.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/c2z-c92998400310b94b.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/f9-038758dcbfe08dab.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/0k-b7df0a74912a42bf.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/sp-352b18edf58f7022.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/og-72a377e4a14147a1.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/b9-4fd36ab02c30bf52.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ec-cbba6ef3015afe55.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/du-3326f9d530cbf97a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/20-e56a5cb6a2f13e42.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ql0-e077eae00b3c661b.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/dsj-cb1e4ddbce4f007b.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/wp-9f5235d335ab624b.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/gc4-c5927d67cdc592a2.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/uw-1408814835e50727.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/7cw-169ae7bb5f04a269.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/z-831ad54ea056549a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/hrn-71934060c9ba1e3a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/w4-a0045409d63046ff.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/jxi-4a01b1e0ce757fd4.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/72-20143cc69e68a0e4.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/xc-f5d51b571cd317cb.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/t-7c4bc4aab3fcad0a.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/code-view-6763778c37f1fa1a.js" defer="defer"></script>
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-css.44b7afb50b7d2b27.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/ql0.a8799830227aa47a.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/z.79ba33c2ec5d04d0.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/xc.13e027137438fd9f.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/t.84009134db3f904a.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/code-view.a18f81c678089bdc.module.css" />

  <link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/8p0n-73376bed45f677a1.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/notifications-subscriptions-menu-77a1e558ed3fc653.js" defer="defer"></script>
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-css.44b7afb50b7d2b27.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/notifications-subscriptions-menu.b5fa61d23b9a02b8.module.css" />


  <title>GitHub - walley/bertik: companion to certik · GitHub</title>



  <meta name="route-pattern" content="/:user_id/:repository" data-turbo-transient>
  <meta name="route-controller" content="files" data-turbo-transient>
  <meta name="route-action" content="disambiguate" data-turbo-transient>
  <meta name="fetch-nonce" content="v2:cf778fc5-331a-615b-fb42-ef43152b8148">

    
  <meta name="current-catalog-service-hash" content="f3abb0cc802f3d7b95fc8762b94bdcb13bf39634c40c357301c4aa1d67a256fb">


  <meta name="request-id" content="2713:316EC5:3DFFE46:394DA33:6AB4F52F" data-pjax-transient="true"/><meta name="html-safe-nonce" content="4c282d20a31873b2d5ba53a196882cc4ec6f282fac5588a86caaae73ec6e983f" data-pjax-transient="true"/><meta name="visitor-payload" content="eyJyZWZlcnJlciI6IiIsInJlcXVlc3RfaWQiOiIyNzEzOjMxNkVDNTozREZGRTQ2OjM5NERBMzM6NkFCNEY1MkYiLCJ2aXNpdG9yX2lkIjoiNjAzNzE1MzIzMDE2NzAxMjY1NSIsInJlZ2lvbl9lZGdlIjoiZnJhIiwicmVnaW9uX3JlbmRlciI6ImZyYSJ9" data-pjax-transient="true"/><meta name="visitor-hmac" content="898191123275ea2234f4e1bbd93f3472374c85c08e53036637cde9c53f2ef631" data-pjax-transient="true"/>


    <meta name="hovercard-subject-tag" content="repository:1372904759" data-turbo-transient>


  <meta name="github-keyboard-shortcuts" content="repository,copilot" data-turbo-transient="true" />
  

  <meta name="selected-link" value="repo_source" data-turbo-transient>
  <link rel="assets" href="https://github.githubassets.com/">

    <meta name="google-site-verification" content="Apib7-x98H0j5cPqHWwSMm6dNU4GmODRoqxLiDzdx9I">

<meta name="octolytics-url" content="https://collector.github.com/github/collect" />





  <meta name="analytics-location" content="/&lt;user-name&gt;/&lt;repo-name&gt;" data-turbo-transient="true" />

  




    <meta name="user-login" content="">

  

    <meta name="viewport" content="width=device-width">

    

      <meta name="description" content="companion to certik. Contribute to walley/bertik development by creating an account on GitHub.">

      <link rel="search" type="application/opensearchdescription+xml" href="/opensearch.xml" title="GitHub">

    <link rel="fluid-icon" href="https://github.com/fluidicon.png" title="GitHub">
    <meta property="fb:app_id" content="1401488693436528">
    <meta name="apple-itunes-app" content="app-id=1477376905, app-argument=https://github.com/walley/bertik" />

      <meta name="twitter:image" content="https://opengraph.githubassets.com/2086730b27903f21290504aafc2b428689fae33638a362c6baf9866396306815/walley/bertik" /><meta name="twitter:site" content="@github" /><meta name="twitter:card" content="summary_large_image" /><meta name="twitter:title" content="GitHub - walley/bertik: companion to certik" /><meta name="twitter:description" content="companion to certik. Contribute to walley/bertik development by creating an account on GitHub." />
  <meta property="og:image" content="https://opengraph.githubassets.com/2086730b27903f21290504aafc2b428689fae33638a362c6baf9866396306815/walley/bertik" /><meta property="og:image:alt" content="companion to certik. Contribute to walley/bertik development by creating an account on GitHub." /><meta property="og:image:width" content="1200" /><meta property="og:image:height" content="600" /><meta property="og:site_name" content="GitHub" /><meta property="og:type" content="object" /><meta property="og:title" content="GitHub - walley/bertik: companion to certik" /><meta property="og:url" content="https://github.com/walley/bertik" /><meta property="og:description" content="companion to certik. Contribute to walley/bertik development by creating an account on GitHub." />
  




      <meta name="hostname" content="github.com">



        <meta name="expected-hostname" content="github.com">


  <meta http-equiv="x-pjax-version" content="e8147740551c17ad4649779964afcf0f7c6e6fbf2d69fcf6f95e5def83e9b343" data-turbo-track="reload">
  <meta http-equiv="x-pjax-csp-version" content="c4a65e47b0c850e1157ae8e66298070851d7e3b558038de5a3b4ff7a93e630ed" data-turbo-track="reload">
  <meta http-equiv="x-pjax-css-version" content="ec4c7b86d7b57240253150b827ea48b04e08d8760dc6553d2a04f62000f3fe0c" data-turbo-track="reload">
  <meta http-equiv="x-pjax-js-version" content="982f0a2c1f3dcde233dc214868b7e87d18cebdb6b2c83b98a936c8f73474bba4" data-turbo-track="reload">

  <meta name="turbo-cache-control" content="no-preview" data-turbo-transient="">

      <meta name="turbo-cache-control" content="no-cache" data-turbo-transient>

    <meta data-hydrostats="publish">

  <meta name="go-import" content="github.com/walley/bertik git https://github.com/walley/bertik.git">

  <meta name="octolytics-dimension-user_id" content="236163" /><meta name="octolytics-dimension-user_login" content="walley" /><meta name="octolytics-dimension-repository_id" content="1372904759" /><meta name="octolytics-dimension-repository_nwo" content="walley/bertik" /><meta name="octolytics-dimension-repository_public" content="true" /><meta name="octolytics-dimension-repository_is_fork" content="false" /><meta name="octolytics-dimension-repository_network_root_id" content="1372904759" /><meta name="octolytics-dimension-repository_network_root_nwo" content="walley/bertik" />
  



    

    <meta name="turbo-body-classes" content="logged-out env-production page-responsive">
  <meta name="disable-turbo" content="false">


  <meta name="browser-stats-url" content="https://api.github.com/_private/browser/stats">


  <meta name="browser-errors-url" content="https://api.github.com/_private/browser/errors">

  <meta name="release" content="30be1065334e97b0e7c320e73f0a6289d47eed36" data-turbo-track="reload">
  <meta name="ui-target" content="full">

  <link rel="mask-icon" href="https://github.githubassets.com/assets/pinned-octocat-093da3e6fa40.svg" color="#000000">
  <link rel="alternate icon" class="js-site-favicon" type="image/png" href="https://github.githubassets.com/favicons/favicon.png">
  <link rel="icon" class="js-site-favicon" type="image/svg+xml" href="https://github.githubassets.com/favicons/favicon.svg" data-base-href="https://github.githubassets.com/favicons/favicon">

<meta name="theme-color" content="#1e2327">
<meta name="color-scheme" content="light dark" />


  <link rel="manifest" href="/manifest.json" crossOrigin="use-credentials">

  </head>

  <body class="logged-out env-production page-responsive" style="word-wrap: break-word;" >
    <div data-turbo-body class="logged-out env-production page-responsive" style="word-wrap: break-word;" >
      <div id="__primerPortalRoot__" style="z-index: 1000; position: absolute; width: 100%;" data-turbo-permanent></div>
      

    <div class="position-relative header-wrapper js-header-wrapper ">
      <a href="#start-of-content" data-skip-target-assigned="false" class="px-2 tmp-py-4 color-bg-accent-emphasis color-fg-on-emphasis show-on-focus js-skip-to-content">Skip to content</a>

      <span data-view-component="true" class="progress-pjax-loader Progress position-fixed width-full">
    <span style="width: 0%;" data-view-component="true" class="Progress-item progress-pjax-loader-bar left-0 top-0 color-bg-accent-emphasis"></span>
</span>      
      <link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/l7j-2ddd8b435dadae28.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/keyboard-shortcuts-dialog-fa6bdebc4a2ffedb.js" fetchpriority="low" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-css.44b7afb50b7d2b27.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/keyboard-shortcuts-dialog.4f138c293c5b3011.module.css" />

<react-partial
  partial-name="keyboard-shortcuts-dialog"
  data-ssr="false"
  data-attempted-ssr="false"
  data-react-profiling="false"
>
  
  <script type="application/json" data-target="react-partial.embeddedData">{"props":{"docsUrl":"https://docs.github.com/get-started/accessibility/keyboard-shortcuts"}}</script>
  <div data-target="react-partial.reactRoot"></div>
</react-partial>





      

          <link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/pw-4f91498888595802.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/app-install-banner-partial-8dfb3ab112a86218.js" fetchpriority="low" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-css.44b7afb50b7d2b27.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/app-install-banner-partial.2b3a47cb3223afb5.module.css" />

<react-partial
  partial-name="app-install-banner-partial"
  data-ssr="false"
  data-attempted-ssr="false"
  data-react-profiling="false"
>
  
  <script type="application/json" data-target="react-partial.embeddedData">{"props":{}}</script>
  <div data-target="react-partial.reactRoot"></div>
</react-partial>


          

                <link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/2l-6cedbc3128429fc7.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/i2-3e1ea9a828b29b0d.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/fz5-23f28c0f388769c9.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/lht-dedd9949bed91081.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/p1-76a75a6081059b19.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/3d-82c347d20a13fb90.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/tua-18c20c09a9dc80b9.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/t1-7addb7a44e70fa3c.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/marketing-header-204eb35de84822f3.js" fetchpriority="low" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-css.44b7afb50b7d2b27.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-brand-css.6a01c236ff527e7e.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/t1.a304baa4c3062c45.module.css" />

<react-partial
  partial-name="marketing-header"
  data-ssr="true"
  data-attempted-ssr="true"
  data-react-profiling="false"
>
  
  <script type="application/json" data-target="react-partial.embeddedData">{"props":{"color_mode":"dark","logged_in":false,"marketing_page":false,"home_path":"/","login_path":"/login?return_to=https%3A%2F%2Fgithub.com%2Fwalley%2Fbertik","signup_path":"/signup?ref_cta=Sign+up\u0026ref_loc=header+logged+out\u0026ref_page=%2F%3Cuser-name%3E%2F%3Crepo-name%3E\u0026source=header-repo\u0026source_repo=walley%2Fbertik","signup_enabled":true,"is_signup_controller":false,"show_search_and_nav":true,"hide_search":false,"private_mode_enabled":false,"should_use_dotcom_links":true,"auth_hydro_click":"{\"event_type\":\"authentication.click\",\"payload\":{\"location_in_page\":\"site header menu\",\"repository_id\":null,\"auth_type\":\"SIGN_UP\",\"originating_url\":\"https://github.com/walley/bertik\",\"user_id\":null}}","auth_hydro_click_hmac":"8544d39fb93586a5df1085a9db27b5ecb10bd034ece7c85b31e0e1b06762d607","overlay":false,"fixed":false}}</script>
  <div data-target="react-partial.reactRoot"><div data-color-mode="dark" data-light-theme="light" data-dark-theme="dark"><header class="MarketingHeader-module__root__Tk7n3 HeaderMktg header-logged-out" role="banner" data-marketing-header="true" data-color-mode="dark" data-light-theme="light" data-dark-theme="dark" data-is-top="true"><h2 class="MarketingHeader-module__visuallyHidden__sqKsl">Navigation Menu</h2><button type="button" class="MarketingHeader-module__backdrop__sw4RU" aria-label="Close navigation menu"></button><div class="MarketingHeader-module__bar__mBSyE"><div class="MarketingHeader-module__topRow__yeury"><div class="MarketingHeader-module__toggleSlot__hDxbh"><button type="button" class="HeaderMenuToggle-module__toggle__i8EiC" aria-label="Toggle navigation" aria-expanded="false"><span class="HeaderMenuToggle-module__toggleBar__jVN0H"></span><span class="HeaderMenuToggle-module__toggleBar__jVN0H"></span><span class="HeaderMenuToggle-module__toggleBar__jVN0H"></span></button></div><a href="/" aria-label="Homepage" class="HeaderLogo-module__logo__UFyHI" data-analytics-event="{&quot;action&quot;:&quot;homepage&quot;,&quot;tag&quot;:&quot;link&quot;,&quot;context&quot;:&quot;logo&quot;,&quot;location&quot;:&quot;header&quot;,&quot;label&quot;:&quot;homepage_link_logo_header&quot;}"><svg data-component="Octicon" aria-hidden="true" focusable="false" class="octicon octicon-mark-github" viewBox="0 0 24 24" width="32" height="32" fill="currentColor" display="inline-block" overflow="visible" style="vertical-align:text-bottom"><path d="M10.226 17.284c-2.965-.36-5.054-2.493-5.054-5.256 0-1.123.404-2.336 1.078-3.144-.292-.741-.247-2.314.09-2.965.898-.112 2.111.36 2.83 1.01.853-.269 1.752-.404 2.853-.404 1.1 0 1.999.135 2.807.382.696-.629 1.932-1.1 2.83-.988.315.606.36 2.179.067 2.942.72.854 1.101 2 1.101 3.167 0 2.763-2.089 4.852-5.098 5.234.763.494 1.28 1.572 1.28 2.807v2.336c0 .674.561 1.056 1.235.786 4.066-1.55 7.255-5.615 7.255-10.646C23.5 6.188 18.334 1 11.978 1 5.62 1 .5 6.188.5 12.545c0 4.986 3.167 9.12 7.435 10.669.606.225 1.19-.18 1.19-.786V20.63a2.9 2.9 0 0 1-1.078.224c-1.483 0-2.359-.808-2.987-2.313-.247-.607-.517-.966-1.034-1.033-.27-.023-.359-.135-.359-.27 0-.27.45-.471.898-.471.652 0 1.213.404 1.797 1.235.45.651.921.943 1.483.943.561 0 .92-.202 1.437-.719.382-.381.674-.718.944-.943"></path></svg></a><div class="AuthCTAs-module__mobileActions__NNzeV"><a class="Primer_Brand__Button-module__Button___scH9Z Primer_Brand__Button-module__Button--subtle___F7pEE Primer_Brand__Button-module__Button--size-small___zQrEw AuthCTAs-module__cta__WpwQq" href="/login?return_to=https%3A%2F%2Fgithub.com%2Fwalley%2Fbertik" data-analytics-event="{&quot;action&quot;:&quot;sign_in&quot;,&quot;tag&quot;:&quot;link&quot;,&quot;context&quot;:&quot;auth_cta&quot;,&quot;location&quot;:&quot;header&quot;,&quot;label&quot;:&quot;sign_in_link_auth_cta_header&quot;}" data-hydro-click="{&quot;event_type&quot;:&quot;authentication.click&quot;,&quot;payload&quot;:{&quot;location_in_page&quot;:&quot;site header menu&quot;,&quot;repository_id&quot;:null,&quot;auth_type&quot;:&quot;SIGN_UP&quot;,&quot;originating_url&quot;:&quot;https://github.com/walley/bertik&quot;,&quot;user_id&quot;:null}}" data-hydro-click-hmac="8544d39fb93586a5df1085a9db27b5ecb10bd034ece7c85b31e0e1b06762d607"><span class="Primer_Brand__Button-module__Button__text___ED0bX"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--100___B2ueX Primer_Brand__Text-module__Text--weight-medium___qJKf_ Primer_Brand__Button-module__Button--label___qrkyz Primer_Brand__Button-module__Button--label-subtle___8ndWH">Sign in</span></span></a><button class="Primer_Brand__Button-module__Button___scH9Z Primer_Brand__Button-module__Button--subtle___F7pEE Primer_Brand__Button-module__Button--size-small___zQrEw HeaderAppearanceSettings-module__trigger__hUheK" type="button" aria-haspopup="dialog" aria-labelledby="_R_3dd_"><span class="Primer_Brand__Button-module__Button__leading-visual___jjtTe" data-testid="Button-leading-visual"><svg data-component="Octicon" focusable="false" aria-hidden="true" class="octicon octicon-sliders Primer_Brand__Button-module__Button__icon-visual____qybb" viewBox="0 0 16 16" width="16" height="16" fill="currentColor" display="inline-block" overflow="visible" style="vertical-align:text-bottom"><path d="M15 2.75a.75.75 0 0 1-.75.75h-4a.75.75 0 0 1 0-1.5h4a.75.75 0 0 1 .75.75Zm-8.5.75v1.25a.75.75 0 0 0 1.5 0v-4a.75.75 0 0 0-1.5 0V2H1.75a.75.75 0 0 0 0 1.5H6.5Zm1.25 5.25a.75.75 0 0 0 0-1.5h-6a.75.75 0 0 0 0 1.5h6ZM15 8a.75.75 0 0 1-.75.75H11.5V10a.75.75 0 1 1-1.5 0V6a.75.75 0 0 1 1.5 0v1.25h2.75A.75.75 0 0 1 15 8Zm-9 5.25v-2a.75.75 0 0 0-1.5 0v1.25H1.75a.75.75 0 0 0 0 1.5H4.5v1.25a.75.75 0 0 0 1.5 0v-2Zm9 0a.75.75 0 0 1-.75.75h-6a.75.75 0 0 1 0-1.5h6a.75.75 0 0 1 .75.75Z"></path></svg></span><span class="Primer_Brand__Button-module__Button__text___ED0bX"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--100___B2ueX Primer_Brand__Text-module__Text--weight-medium___qJKf_ Primer_Brand__Button-module__Button--label___qrkyz Primer_Brand__Button-module__Button--label-subtle___8ndWH"></span></span></button><div class="Primer_Brand__Tooltip-module__Tooltip___0Eipx" data-direction="s" aria-hidden="true" id="_R_3dd_">Appearance settings</div></div></div><div class="MarketingHeader-module__menu__GIy3y"><div class="MarketingHeader-module__menuWrapper__owstH"><nav class="MarketingNavigation-module__nav__W0KYY" aria-label="Global"><ul class="MarketingNavigation-module__list__tFbMb"><li><div class="NavDropdown-module__container__l2YeI"><button type="button" class="NavDropdown-module__button__PEHWX" aria-expanded="false" aria-controls="_R_nd_">Platform<svg data-component="Octicon" aria-hidden="true" focusable="false" class="octicon octicon-triangle-right NavDropdown-module__buttonIcon__Tkl8_" viewBox="0 0 16 16" width="16" height="16" fill="currentColor" display="inline-block" overflow="visible" style="vertical-align:text-bottom"><path d="m6.427 4.427 3.396 3.396a.25.25 0 0 1 0 .354l-3.396 3.396A.25.25 0 0 1 6 11.396V4.604a.25.25 0 0 1 .427-.177Z"></path></svg></button><div id="_R_nd_" class="NavDropdown-module__dropdown__xm1jd"><ul class="NavDropdown-module__list__zuCgG"><li><div class="NavGroup-module__group__W8SqJ"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--monospace___QXHDQ Primer_Brand__Text-module__Text--muted___rE6mh Primer_Brand__Text-module__Text--100___B2ueX Primer_Brand__Text-module__Text--weight-medium___qJKf_ NavGroup-module__title__Wzxz2" id="_R_5knd_">AI CODE CREATION</span><ul class="NavGroup-module__list__UCOFy" aria-labelledby="_R_5knd_"><li><a class="Primer_Brand__Link-module__Link___lF11y Primer_Brand__Link-module__Link--default___VRVW0 NavLink-module__link__EG3d4" href="https://github.com/features/copilot" data-analytics-event="{&quot;action&quot;:&quot;github_copilot&quot;,&quot;tag&quot;:&quot;link&quot;,&quot;context&quot;:&quot;platform&quot;,&quot;location&quot;:&quot;navbar&quot;,&quot;label&quot;:&quot;github_copilot_link_platform_navbar&quot;}"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS Primer_Brand__Link-module__Link--label___jM8Ty"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS Primer_Brand__Text-module__Text--weight-medium___qJKf_ NavLink-module__title__Q7t0p"><svg data-component="Octicon" aria-hidden="true" focusable="false" class="octicon octicon-copilot NavLink-module__icon__ltGNM" viewBox="0 0 16 16" width="16" height="16" fill="currentColor" display="inline-block" overflow="visible" style="vertical-align:text-bottom"><path d="M7.998 15.035c-4.562 0-7.873-2.914-7.998-3.749V9.338c.085-.628.677-1.686 1.588-2.065.013-.07.024-.143.036-.218.029-.183.06-.384.126-.612-.201-.508-.254-1.084-.254-1.656 0-.87.128-1.769.693-2.484.579-.733 1.494-1.124 2.724-1.261 1.206-.134 2.262.034 2.944.765.05.053.096.108.139.165.044-.057.094-.112.143-.165.682-.731 1.738-.899 2.944-.765 1.23.137 2.145.528 2.724 1.261.566.715.693 1.614.693 2.484 0 .572-.053 1.148-.254 1.656.066.228.098.429.126.612.012.076.024.148.037.218.924.385 1.522 1.471 1.591 2.095v1.872c0 .766-3.351 3.795-8.002 3.795Zm0-1.485c2.28 0 4.584-1.11 5.002-1.433V7.862l-.023-.116c-.49.21-1.075.291-1.727.291-1.146 0-2.059-.327-2.71-.991A3.222 3.222 0 0 1 8 6.303a3.24 3.24 0 0 1-.544.743c-.65.664-1.563.991-2.71.991-.652 0-1.236-.081-1.727-.291l-.023.116v4.255c.419.323 2.722 1.433 5.002 1.433ZM6.762 2.83c-.193-.206-.637-.413-1.682-.297-1.019.113-1.479.404-1.713.7-.247.312-.369.789-.369 1.554 0 .793.129 1.171.308 1.371.162.181.519.379 1.442.379.853 0 1.339-.235 1.638-.54.315-.322.527-.827.617-1.553.117-.935-.037-1.395-.241-1.614Zm4.155-.297c-1.044-.116-1.488.091-1.681.297-.204.219-.359.679-.242 1.614.091.726.303 1.231.618 1.553.299.305.784.54 1.638.54.922 0 1.28-.198 1.442-.379.179-.2.308-.578.308-1.371 0-.765-.123-1.242-.37-1.554-.233-.296-.693-.587-1.713-.7Z"></path><path d="M6.25 9.037a.75.75 0 0 1 .75.75v1.501a.75.75 0 0 1-1.5 0V9.787a.75.75 0 0 1 .75-.75Zm4.25.75v1.501a.75.75 0 0 1-1.5 0V9.787a.75.75 0 0 1 1.5 0Z"></path></svg>GitHub Copilot</span><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--muted___rE6mh Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS NavLink-module__subtitle__X4gkW">Write better code with AI</span></span></a></li><li><a class="Primer_Brand__Link-module__Link___lF11y Primer_Brand__Link-module__Link--default___VRVW0 NavLink-module__link__EG3d4" href="https://github.com/features/ai/github-app" data-analytics-event="{&quot;action&quot;:&quot;github_copilot_app&quot;,&quot;tag&quot;:&quot;link&quot;,&quot;context&quot;:&quot;platform&quot;,&quot;location&quot;:&quot;navbar&quot;,&quot;label&quot;:&quot;github_copilot_app_link_platform_navbar&quot;}"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS Primer_Brand__Link-module__Link--label___jM8Ty"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS Primer_Brand__Text-module__Text--weight-medium___qJKf_ NavLink-module__title__Q7t0p"><svg data-component="Octicon" aria-hidden="true" focusable="false" class="octicon octicon-mark-github NavLink-module__icon__ltGNM" viewBox="0 0 16 16" width="16" height="16" fill="currentColor" display="inline-block" overflow="visible" style="vertical-align:text-bottom"><path d="M6.766 11.328c-2.063-.25-3.516-1.734-3.516-3.656 0-.781.281-1.625.75-2.188-.203-.515-.172-1.609.063-2.062.625-.078 1.468.25 1.968.703.594-.187 1.219-.281 1.985-.281.765 0 1.39.094 1.953.265.484-.437 1.344-.765 1.969-.687.218.422.25 1.515.046 2.047.5.593.766 1.39.766 2.203 0 1.922-1.453 3.375-3.547 3.64.531.344.89 1.094.89 1.954v1.625c0 .468.391.734.86.547C13.781 14.359 16 11.53 16 8.03 16 3.61 12.406 0 7.984 0 3.563 0 0 3.61 0 8.031a7.88 7.88 0 0 0 5.172 7.422c.422.156.828-.125.828-.547v-1.25c-.219.094-.5.156-.75.156-1.031 0-1.64-.562-2.078-1.609-.172-.422-.36-.672-.719-.719-.187-.015-.25-.093-.25-.187 0-.188.313-.328.625-.328.453 0 .844.281 1.25.86.313.452.64.655 1.031.655s.641-.14 1-.5c.266-.265.47-.5.657-.656"></path></svg>GitHub Copilot app</span><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--muted___rE6mh Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS NavLink-module__subtitle__X4gkW">Direct agents from issue to merge</span></span></a></li><li><a class="Primer_Brand...[truncated, total 240602 bytes]
```

_Thinking:_ The fetched HTML is very large. I can see the meta description tag for bertik: `<meta name="description" content="companion to certik. Contribute to walley/bertik development by creating an account on GitHub.">`

Let me try to extract the description meta tags from each page. Actually, let me try a different approach - I'll search for the description meta tag pattern in the fetched content. But the content is huge. Let me try to use a more targeted approach - maybe I can fetch just the raw README or use the GitHub API with different parameters.

Actually, let me try one more thing - maybe I can use the GitHub API with a different approach. Let me try without the `/repos` path, maybe just the raw content API.

Actually, let me take a step back. Since I can't easily get the GitHub API data due to 403 errors, and the HTML pages are very large with lots of scripts, let me try to extract just the meta description tags which I can see in the head section.

From what I've seen:
- certik: description is "certik bertik. Contribute to walley/certik development by creating an account on GitHub."
- bertik: description is "companion to certik. Contribute to walley/bertik development by creating an account on GitHub."

Let me try to fetch rustama as well, and then I'll create the HTML page with the information I have.

**Assistant:** 


**Tool Call:** `fetch_url`
```
{"url":"https://github.com/walley/rustama"}
```

**Tool Result:** `fetch_url`
```




  

<!DOCTYPE html>
<html
  lang="en"
  
  data-color-mode="auto" data-light-theme="light" data-dark-theme="dark"
  data-a11y-animated-images="system" data-a11y-link-underlines="true"
  
  >




  <head>
    <meta charset="utf-8">
  <link rel="dns-prefetch" href="https://github.githubassets.com">
  <link rel="dns-prefetch" href="https://avatars.githubusercontent.com">
  <link rel="dns-prefetch" href="https://github-cloud.s3.amazonaws.com">
  <link rel="dns-prefetch" href="https://user-images.githubusercontent.com/">
  <link rel="preconnect" href="https://github.githubassets.com" crossorigin>
  <link rel="preconnect" href="https://avatars.githubusercontent.com">

<script type="importmap">{"imports":{"react":"https://github.githubassets.com/assets/react-e27d1b3e03961e68.js","react-dom":"https://github.githubassets.com/assets/react-dom-e5fd46a22d5c4058.js","react-dom/client":"https://github.githubassets.com/assets/react-dom-client-1b4a3ee065998cea.js","react-is":"https://github.githubassets.com/assets/react-is-e0b593954b4706d8.js","react-reconciler":"https://github.githubassets.com/assets/react-reconciler-8e99e505c4429605.js","react/compiler-runtime":"https://github.githubassets.com/assets/react-compiler-runtime-4610bd6d3de9c049.js","react/jsx-dev-runtime":"https://github.githubassets.com/assets/react-jsx-dev-runtime-ea55d68667d559e5.js","react/jsx-runtime":"https://github.githubassets.com/assets/react-jsx-runtime-4915cb0f5b3aff04.js","scheduler":"https://github.githubassets.com/assets/scheduler-58b860b049ca307c.js"}}</script>
<meta name="react-profiling" content="0" data-turbo-transient="true" />
<meta name="react-import-map" content="react,react-dom,react-dom/client,react-dom/profiling,react-is,react-reconciler,react/compiler-runtime,react/jsx-dev-runtime,react/jsx-runtime,scheduler@777f63f87cd0" data-turbo-track="reload" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-e27d1b3e03961e68.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-compiler-runtime-4610bd6d3de9c049.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/scheduler-58b860b049ca307c.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-dom-e5fd46a22d5c4058.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-dom-client-1b4a3ee065998cea.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-is-e0b593954b4706d8.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-jsx-runtime-4915cb0f5b3aff04.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-reconciler-8e99e505c4429605.js" />

  


  <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/light-99f877e9ddfc0e51.css" /><link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/light_high_contrast-48fdd0811afbab3c.css" /><link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/dark-79ad2ace604703b3.css" /><link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/dark_high_contrast-24484a076f02295f.css" /><link data-color-theme="light" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light-99f877e9ddfc0e51.css" /><link data-color-theme="light_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light_high_contrast-48fdd0811afbab3c.css" /><link data-color-theme="light_colorblind" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light_colorblind-f4bf1142976e4bbf.css" /><link data-color-theme="light_colorblind_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light_colorblind_high_contrast-f661b49995ba0bd8.css" /><link data-color-theme="light_tritanopia" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light_tritanopia-0b38d22346321c92.css" /><link data-color-theme="light_tritanopia_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/light_tritanopia_high_contrast-f1c62c9e70259b9f.css" /><link data-color-theme="dark" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark-79ad2ace604703b3.css" /><link data-color-theme="dark_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_high_contrast-24484a076f02295f.css" /><link data-color-theme="dark_colorblind" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_colorblind-f50cacf0a86b9929.css" /><link data-color-theme="dark_colorblind_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_colorblind_high_contrast-e61d4f4ca17852c2.css" /><link data-color-theme="dark_tritanopia" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_tritanopia-39c10993d5603fac.css" /><link data-color-theme="dark_tritanopia_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_tritanopia_high_contrast-73236c840c0c7d90.css" /><link data-color-theme="dark_dimmed" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_dimmed-0de76f07cc035b10.css" /><link data-color-theme="dark_dimmed_high_contrast" crossorigin="anonymous" media="all" rel="stylesheet" data-href="https://github.githubassets.com/assets/dark_dimmed_high_contrast-fd1500c8744e40d6.css" />

  <style type="text/css">
    :root {
      --tab-size-preference: 4;
    }

    pre, code {
      tab-size: var(--tab-size-preference);
    }
  </style>

    <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-primitives-ed9ca172356fd545.css" />
    <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-1d2c7f7b52a6068b.css" />
    <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/global-54ba76e934a49d7c.css" />
    <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/github-456bd5deb85c7ecd.css" />
  <link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/repository-11ee8a031c040c1a.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/code-2d56bdb0166c0238.css" />

  

  <script type="application/json" id="client-env">{"locale":"en","featureFlags":["actions_enable_background_steps","actions_new_hosted_runner_image_select_sizes_and_versions","activity_diff_file_tree","activity_repos_file_tree","activity_repos_overview_header","activity_repos_overview_sidebar","agent_author_search_expansion","agent_author_search_expansion_ui_pulls","alternate_user_config_repo","async_conversion_coverage_enabled","billing_billable_licenses_cost_center_bucket_fix","billing_budget_expiration","billing_cost_center_list_assigned_resources","billing_discount_threshold_notification","code_quality_enablement_banner_targeting","code_quality_remove_preview","code_view_raf_sticky_lines","codespaces_prebuild_region_target_update","coding_agent_third_party_model_ui","copilot_agent_snippy","copilot_api_agentic_issue_marshal_yaml","copilot_automation_repo_mcp_servers","copilot_automations_pagination","copilot_chat_auto_mode_v2","copilot_chat_clear_model_selection_for_default_change","copilot_chat_vision_dotcom_chat_ga_gate","copilot_css_textarea_autosize","copilot_custom_copilots","copilot_custom_copilots_feature_preview","copilot_duplicate_thread","copilot_extensions_removal_on_marketplace","copilot_fix_failed_workflows_all_skus","copilot_hide_hovercard","copilot_immersive_task_hyperlinking","copilot_mc_cli_resume_any_users_task","copilot_mission_control_agent_merge_fix_ci","copilot_mission_control_agent_merge_resolve_conflicts","copilot_mission_control_early_stop","copilot_mission_control_environment_list_icons","copilot_mission_control_managed_sandbox_environments","copilot_mission_control_needs_attention","copilot_mission_control_reasoning_effort","copilot_mission_control_sandbox_client_side_clone","copilot_mission_control_sandbox_remote_bypass","copilot_mission_control_session_filters","copilot_mission_control_task_alive_updates","copilot_mission_control_task_sharing","copilot_org_policy_page_focus_mode","copilot_share_active_subthread","copilot_spaces_ga","copilot_spaces_individual_policies_ga","copilot_swe_agent_authorization_status_ui","copilot_swe_agent_automation_resource_scoped_writes","copilot_swe_agent_hide_model_picker_if_only_auto","copilot_swe_agent_issue_comment_trigger","copilot_swe_agent_pr_comment_model_picker","copilot_swe_agent_pull_request_comment_trigger","copilot_swe_agent_pull_request_merged_trigger","copilot_swe_agent_pull_request_opened_trigger","copilot_swe_agent_pull_request_synchronize_trigger","copilot_swe_agent_use_subagents","copilot_task_api_github_rest_style","copilot_task_scoped_alive_channel","copilot_token_based_billing","copilot_unconfigured_is_inherited","copilot_user_can_upgrade_plan_field","copilot_workbench_sunset_redirect","dashboard_indexeddb_caching","dashboard_lists_max_age_filter","dashboard_universe_2025_feedback_dialog","fgpat_permissions_selector_redesign","glc_code_quality_repo_settings_workflow_config","hide_github_models_ui","hide_groups_list_for_few_groups","hyperspace_2025_logged_out_batch_1","hyperspace_2025_logged_out_batch_2","hyperspace_2025_logged_out_batch_3","in_product_messaging_datadog_monitoring","ipm_ubb_individual_budget_banner","issue_fields_multi_select","issue_inline_avatars","issue_pinned_views","issue_relative_time_micro","issues_dashboard_sso_structured_errors","issues_expanded_file_types","issues_hide_closed_sub_issues","issues_lazy_load_comment_box_suggestions","issues_react_chrome_container_query_fix","issues_relates_to_projects_search","labels_archiving","labels_archiving_info","landing_pages_ninetailed","lifecycle_label_name_updates","marketing_cookie_consent_banner","marketing_pages_search_explore_provider","memex_default_issue_create_repository","memex_lazy_hydrate_agent_tasks","memex_live_update_hovercard","memex_mwl_filter_field_delimiter","memex_remove_deprecated_type_issue","merge_queue_restricted_pushers_warning","merge_status_checks_refetch_dedupe","merge_status_header_feedback","mergeability_refetch_dedupe","octocaptcha_origin_optimization","primer_react_css_anchor_positioning","primer_react_merged_forwarded_refs","prs_copilot_app_open_action","prs_css_anchor_positioning","prs_thread_reply_as_primary_action","pull_request_copilot_attribution_header","pull_request_overview_panel_edit_description","pull_request_persister","pull_request_stacks_feedback_dialog","pull_request_virtualization_image_estimate","pull_request_virtualization_loader_batching","pull_request_virtualization_scroll_compensation","pull_request_virtualization_scroll_intent","quick_search_lazy_suggestions","react_blob_isolate_code_lines","react_blob_ssr_content_visibility","react_data_router_tanstack_allowed","react_query_props_with_key","react_sandbox_future_tanstack","repo_app_turbo","repo_issues_sidebar_layout","repo_pulls_dashboard_declutter","repo_pulls_dashboard_ga","repo_pulls_dashboard_persistence","repo_pulls_dashboard_sidebar_links","repos_contributors_limited_default_range","review_involves_filter","rule_ignored_file_paths","rulesets_actor_list_editor","sample_network_conn_type","sanitize_preset_flash_error","sanitize_preset_validation_message","security_center_artifact_filters_popover","see_who_reacted","semantic_similarity_duplicate_issue_detection","session_logs_ungroup_reasoning_text","set_sha256_on_repo_creation_form","site_banner_desktop_copilot_app","site_ghca_pixel_mona","site_github_app_ga_page","site_github_app_ga_page_highlight","site_github_app_mobile_native_share","site_global_banner_dev_days_attendee","site_global_banner_learn_copilot_sdk","site_global_nav_spark_models_removed","speculation_rules_ui_service","suggest_custom_property_values_copilot","suppress_automated_browser_vitals","swp_forms_disable_octocaptcha","thread_resolution_reason","ui_service_referrer_metrics","update_issue_suggestions","user_code_paste_ux","viewscreen_sandbox","warn_inaccessible_attachments","webp_support","workstream_plugin_bootstrap"],"githubDomain":"https://github.com","copilotApiOverrideUrl":"https://api.githubcopilot.com","cmcApiUrl":"https://api.github.com/cmc_internal/api"}</script>
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/high-contrast-cookie-e3d808ee18eb9784.js"></script>
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/wp-runtime-05d1942afe214994.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/app-foundation-74e9766bd696d26a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/app-runtime-7d377d1f27bbad23.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/fetch-utilities-9a5c31efd5313aa8.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ser-cd1b421ad0ad7ed5.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/environment-b3a091d0025aba04.js" defer="defer"></script>
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/app-runtime.9246aa20d02bb90b.module.css" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/catalyst-044fa3a2acdaf0a9.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/selector-observer-e88088f989b27670.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/relative-time-element-7cdf4e0db98b997c.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/by-6b3c07383371bf58.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ja9-36f036c13c2e3a4c.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/kw-866fd9513ae95106.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/r1-b075aab3204f32f9.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/jz5-7c5d1669717041f8.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/hj-5126390ff433704d.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/j05-dce4d588268f0f7e.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/github-elements-c45a71167e34440f.js" defer="defer"></script>
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/element-registry-db7d32eddf19b9b9.js" defer="defer"></script>
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/runtime-helpers-6df03a157bacbc44.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/aria-live-76b18916d0c8ec4d.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/hotkey-5109c77d7ac61078.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/react-core-d27b938e851ccb49.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/04-b4570e9cc17d8b62.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ur-024971cb63acad59.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/4t-364e19c63d726e9a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/6ny-c969fbae697a356f.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/yhq-3a209035a30b1a09.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/z1v-da5b33b877bfa358.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/wo-df202594a5bf5f61.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/9y-ebc4195ed57c6001.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/j0-41a4cf04cbd99503.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/0p-81491ed0d9d666e2.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/behaviors-e280187a9767d3e6.js" defer="defer"></script>
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/react-core.5c6ee197c091b384.module.css" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/n9-97e2e5539969aaaf.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/codespaces-a427ff8c1198f956.js" defer="defer"></script>
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ic-991f04433f366e7b.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/repositories-b85d65c9fda8a709.js" defer="defer"></script>
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/code-menu-d49e4b9c111b9b3e.js" defer="defer"></script>
  
  <link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/primer-react-3ca70c6e707ba409.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ncx-9a860680a5f230b9.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/j6-363f0ebf4f62f310.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/t7l-bf6f21d7e7a43c35.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/4u-27f74cfc703e1977.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/wsr-5fd3071e5a9061ab.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/0n-ecadb2ffe6b01ba2.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/s07-3fae157786a7acb1.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/n4u-6317ce1c31aed1db.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/j8s-d90ce3a169b741ed.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/la-d2cd96f070e6b52a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/q61-80cd2b936a4f27b6.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/fj8-b6b4dd5c8ee28797.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/w4q-134941577ca76346.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/qhu-cfdd732196b44b49.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/gzu-b9f61517dfb069e5.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/bi0-a78ba744687848c2.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/on-c7e4c90af6ffd527.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/b1-04bc5715b6dd3e67.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/0l5-2c7e8df6e59adc66.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/c2z-c92998400310b94b.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/f9-038758dcbfe08dab.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/0k-b7df0a74912a42bf.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/sp-352b18edf58f7022.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/og-72a377e4a14147a1.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/b9-4fd36ab02c30bf52.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ec-cbba6ef3015afe55.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/du-3326f9d530cbf97a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/20-e56a5cb6a2f13e42.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/ql0-e077eae00b3c661b.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/dsj-cb1e4ddbce4f007b.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/wp-9f5235d335ab624b.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/gc4-c5927d67cdc592a2.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/uw-1408814835e50727.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/7cw-169ae7bb5f04a269.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/z-831ad54ea056549a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/hrn-71934060c9ba1e3a.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/w4-a0045409d63046ff.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/jxi-4a01b1e0ce757fd4.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/72-20143cc69e68a0e4.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/xc-f5d51b571cd317cb.js" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/t-7c4bc4aab3fcad0a.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/code-view-6763778c37f1fa1a.js" defer="defer"></script>
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-css.44b7afb50b7d2b27.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/ql0.a8799830227aa47a.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/z.79ba33c2ec5d04d0.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/xc.13e027137438fd9f.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/t.84009134db3f904a.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/code-view.a18f81c678089bdc.module.css" />

  <link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/8p0n-73376bed45f677a1.js" />
<script crossorigin="anonymous" type="module" src="https://github.githubassets.com/assets/notifications-subscriptions-menu-77a1e558ed3fc653.js" defer="defer"></script>
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-css.44b7afb50b7d2b27.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/notifications-subscriptions-menu.b5fa61d23b9a02b8.module.css" />


  <title>GitHub - walley/Rustama: rust tui ollama client with agentic mode, tools, internal terminal · GitHub</title>



  <meta name="route-pattern" content="/:user_id/:repository" data-turbo-transient>
  <meta name="route-controller" content="files" data-turbo-transient>
  <meta name="route-action" content="disambiguate" data-turbo-transient>
  <meta name="fetch-nonce" content="v2:6246e302-e93b-bbd9-d236-8e146e92795b">

    
  <meta name="current-catalog-service-hash" content="f3abb0cc802f3d7b95fc8762b94bdcb13bf39634c40c357301c4aa1d67a256fb">


  <meta name="request-id" content="2711:323CCF:4024337:3B2FF9F:6AB4F56B" data-pjax-transient="true"/><meta name="html-safe-nonce" content="7df5a6329b3fb29e2071691a192ee121540b77b22483314611715e7cea32439c" data-pjax-transient="true"/><meta name="visitor-payload" content="eyJyZWZlcnJlciI6IiIsInJlcXVlc3RfaWQiOiIyNzExOjMyM0NDRjo0MDI0MzM3OjNCMkZGOUY6NkFCNEY1NkIiLCJ2aXNpdG9yX2lkIjoiNDEwODEwNTA2MDQyNjUxMTcyMyIsInJlZ2lvbl9lZGdlIjoiZnJhIiwicmVnaW9uX3JlbmRlciI6ImZyYSJ9" data-pjax-transient="true"/><meta name="visitor-hmac" content="25cb63d03d820931a0f5a93ea91df6c7d3ed029218df993254ee28c768cb8349" data-pjax-transient="true"/>


    <meta name="hovercard-subject-tag" content="repository:1293254996" data-turbo-transient>


  <meta name="github-keyboard-shortcuts" content="repository,copilot" data-turbo-transient="true" />
  

  <meta name="selected-link" value="repo_source" data-turbo-transient>
  <link rel="assets" href="https://github.githubassets.com/">

    <meta name="google-site-verification" content="Apib7-x98H0j5cPqHWwSMm6dNU4GmODRoqxLiDzdx9I">

<meta name="octolytics-url" content="https://collector.github.com/github/collect" />





  <meta name="analytics-location" content="/&lt;user-name&gt;/&lt;repo-name&gt;" data-turbo-transient="true" />

  




    <meta name="user-login" content="">

  

    <meta name="viewport" content="width=device-width">

    

      <meta name="description" content="rust tui ollama client with agentic mode, tools, internal terminal - walley/Rustama">

      <link rel="search" type="application/opensearchdescription+xml" href="/opensearch.xml" title="GitHub">

    <link rel="fluid-icon" href="https://github.com/fluidicon.png" title="GitHub">
    <meta property="fb:app_id" content="1401488693436528">
    <meta name="apple-itunes-app" content="app-id=1477376905, app-argument=https://github.com/walley/rustama" />

      <meta name="twitter:image" content="https://opengraph.githubassets.com/45e2078d3423fae205081f2dc294a9b95231a4292bca0fed4e1dfdd29d684f19/walley/Rustama" /><meta name="twitter:site" content="@github" /><meta name="twitter:card" content="summary_large_image" /><meta name="twitter:title" content="GitHub - walley/Rustama: rust tui ollama client with agentic mode, tools, internal terminal" /><meta name="twitter:description" content="rust tui ollama client with agentic mode, tools, internal terminal - walley/Rustama" />
  <meta property="og:image" content="https://opengraph.githubassets.com/45e2078d3423fae205081f2dc294a9b95231a4292bca0fed4e1dfdd29d684f19/walley/Rustama" /><meta property="og:image:alt" content="rust tui ollama client with agentic mode, tools, internal terminal - walley/Rustama" /><meta property="og:image:width" content="1200" /><meta property="og:image:height" content="600" /><meta property="og:site_name" content="GitHub" /><meta property="og:type" content="object" /><meta property="og:title" content="GitHub - walley/Rustama: rust tui ollama client with agentic mode, tools, internal terminal" /><meta property="og:url" content="https://github.com/walley/Rustama" /><meta property="og:description" content="rust tui ollama client with agentic mode, tools, internal terminal - walley/Rustama" />
  




      <meta name="hostname" content="github.com">



        <meta name="expected-hostname" content="github.com">


  <meta http-equiv="x-pjax-version" content="e8147740551c17ad4649779964afcf0f7c6e6fbf2d69fcf6f95e5def83e9b343" data-turbo-track="reload">
  <meta http-equiv="x-pjax-csp-version" content="c4a65e47b0c850e1157ae8e66298070851d7e3b558038de5a3b4ff7a93e630ed" data-turbo-track="reload">
  <meta http-equiv="x-pjax-css-version" content="ec4c7b86d7b57240253150b827ea48b04e08d8760dc6553d2a04f62000f3fe0c" data-turbo-track="reload">
  <meta http-equiv="x-pjax-js-version" content="982f0a2c1f3dcde233dc214868b7e87d18cebdb6b2c83b98a936c8f73474bba4" data-turbo-track="reload">

  <meta name="turbo-cache-control" content="no-preview" data-turbo-transient="">

      <meta name="turbo-cache-control" content="no-cache" data-turbo-transient>

    <meta data-hydrostats="publish">

  <meta name="go-import" content="github.com/walley/Rustama git https://github.com/walley/Rustama.git">

  <meta name="octolytics-dimension-user_id" content="236163" /><meta name="octolytics-dimension-user_login" content="walley" /><meta name="octolytics-dimension-repository_id" content="1293254996" /><meta name="octolytics-dimension-repository_nwo" content="walley/Rustama" /><meta name="octolytics-dimension-repository_public" content="true" /><meta name="octolytics-dimension-repository_is_fork" content="false" /><meta name="octolytics-dimension-repository_network_root_id" content="1293254996" /><meta name="octolytics-dimension-repository_network_root_nwo" content="walley/Rustama" />
  



    

    <meta name="turbo-body-classes" content="logged-out env-production page-responsive">
  <meta name="disable-turbo" content="false">


  <meta name="browser-stats-url" content="https://api.github.com/_private/browser/stats">


  <meta name="browser-errors-url" content="https://api.github.com/_private/browser/errors">

  <meta name="release" content="30be1065334e97b0e7c320e73f0a6289d47eed36" data-turbo-track="reload">
  <meta name="ui-target" content="full">

  <link rel="mask-icon" href="https://github.githubassets.com/assets/pinned-octocat-093da3e6fa40.svg" color="#000000">
  <link rel="alternate icon" class="js-site-favicon" type="image/png" href="https://github.githubassets.com/favicons/favicon.png">
  <link rel="icon" class="js-site-favicon" type="image/svg+xml" href="https://github.githubassets.com/favicons/favicon.svg" data-base-href="https://github.githubassets.com/favicons/favicon">

<meta name="theme-color" content="#1e2327">
<meta name="color-scheme" content="light dark" />


  <link rel="manifest" href="/manifest.json" crossOrigin="use-credentials">

  </head>

  <body class="logged-out env-production page-responsive" style="word-wrap: break-word;" >
    <div data-turbo-body class="logged-out env-production page-responsive" style="word-wrap: break-word;" >
      <div id="__primerPortalRoot__" style="z-index: 1000; position: absolute; width: 100%;" data-turbo-permanent></div>
      

    <div class="position-relative header-wrapper js-header-wrapper ">
      <a href="#start-of-content" data-skip-target-assigned="false" class="px-2 tmp-py-4 color-bg-accent-emphasis color-fg-on-emphasis show-on-focus js-skip-to-content">Skip to content</a>

      <span data-view-component="true" class="progress-pjax-loader Progress position-fixed width-full">
    <span style="width: 0%;" data-view-component="true" class="Progress-item progress-pjax-loader-bar left-0 top-0 color-bg-accent-emphasis"></span>
</span>      
      <link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/l7j-2ddd8b435dadae28.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/keyboard-shortcuts-dialog-fa6bdebc4a2ffedb.js" fetchpriority="low" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-css.44b7afb50b7d2b27.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/keyboard-shortcuts-dialog.4f138c293c5b3011.module.css" />

<react-partial
  partial-name="keyboard-shortcuts-dialog"
  data-ssr="false"
  data-attempted-ssr="false"
  data-react-profiling="false"
>
  
  <script type="application/json" data-target="react-partial.embeddedData">{"props":{"docsUrl":"https://docs.github.com/get-started/accessibility/keyboard-shortcuts"}}</script>
  <div data-target="react-partial.reactRoot"></div>
</react-partial>





      

          <link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/pw-4f91498888595802.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/app-install-banner-partial-8dfb3ab112a86218.js" fetchpriority="low" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-css.44b7afb50b7d2b27.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/app-install-banner-partial.2b3a47cb3223afb5.module.css" />

<react-partial
  partial-name="app-install-banner-partial"
  data-ssr="false"
  data-attempted-ssr="false"
  data-react-profiling="false"
>
  
  <script type="application/json" data-target="react-partial.embeddedData">{"props":{}}</script>
  <div data-target="react-partial.reactRoot"></div>
</react-partial>


          

                <link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/2l-6cedbc3128429fc7.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/i2-3e1ea9a828b29b0d.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/fz5-23f28c0f388769c9.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/lht-dedd9949bed91081.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/p1-76a75a6081059b19.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/3d-82c347d20a13fb90.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/tua-18c20c09a9dc80b9.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/t1-7addb7a44e70fa3c.js" fetchpriority="low" />
<link crossorigin="anonymous" rel="modulepreload" href="https://github.githubassets.com/assets/marketing-header-204eb35de84822f3.js" fetchpriority="low" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-css.44b7afb50b7d2b27.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/primer-react-brand-css.6a01c236ff527e7e.module.css" />
<link crossorigin="anonymous" media="all" rel="stylesheet" href="https://github.githubassets.com/assets/t1.a304baa4c3062c45.module.css" />

<react-partial
  partial-name="marketing-header"
  data-ssr="true"
  data-attempted-ssr="true"
  data-react-profiling="false"
>
  
  <script type="application/json" data-target="react-partial.embeddedData">{"props":{"color_mode":"dark","logged_in":false,"marketing_page":false,"home_path":"/","login_path":"/login?return_to=https%3A%2F%2Fgithub.com%2Fwalley%2Frustama","signup_path":"/signup?ref_cta=Sign+up\u0026ref_loc=header+logged+out\u0026ref_page=%2F%3Cuser-name%3E%2F%3Crepo-name%3E\u0026source=header-repo\u0026source_repo=walley%2FRustama","signup_enabled":true,"is_signup_controller":false,"show_search_and_nav":true,"hide_search":false,"private_mode_enabled":false,"should_use_dotcom_links":true,"auth_hydro_click":"{\"event_type\":\"authentication.click\",\"payload\":{\"location_in_page\":\"site header menu\",\"repository_id\":null,\"auth_type\":\"SIGN_UP\",\"originating_url\":\"https://github.com/walley/rustama\",\"user_id\":null}}","auth_hydro_click_hmac":"6ed2305d54d33e9f50a0c05c4a926f9cb86d06d219af895e77af224601d4f14c","overlay":false,"fixed":false}}</script>
  <div data-target="react-partial.reactRoot"><div data-color-mode="dark" data-light-theme="light" data-dark-theme="dark"><header class="MarketingHeader-module__root__Tk7n3 HeaderMktg header-logged-out" role="banner" data-marketing-header="true" data-color-mode="dark" data-light-theme="light" data-dark-theme="dark" data-is-top="true"><h2 class="MarketingHeader-module__visuallyHidden__sqKsl">Navigation Menu</h2><button type="button" class="MarketingHeader-module__backdrop__sw4RU" aria-label="Close navigation menu"></button><div class="MarketingHeader-module__bar__mBSyE"><div class="MarketingHeader-module__topRow__yeury"><div class="MarketingHeader-module__toggleSlot__hDxbh"><button type="button" class="HeaderMenuToggle-module__toggle__i8EiC" aria-label="Toggle navigation" aria-expanded="false"><span class="HeaderMenuToggle-module__toggleBar__jVN0H"></span><span class="HeaderMenuToggle-module__toggleBar__jVN0H"></span><span class="HeaderMenuToggle-module__toggleBar__jVN0H"></span></button></div><a href="/" aria-label="Homepage" class="HeaderLogo-module__logo__UFyHI" data-analytics-event="{&quot;action&quot;:&quot;homepage&quot;,&quot;tag&quot;:&quot;link&quot;,&quot;context&quot;:&quot;logo&quot;,&quot;location&quot;:&quot;header&quot;,&quot;label&quot;:&quot;homepage_link_logo_header&quot;}"><svg data-component="Octicon" aria-hidden="true" focusable="false" class="octicon octicon-mark-github" viewBox="0 0 24 24" width="32" height="32" fill="currentColor" display="inline-block" overflow="visible" style="vertical-align:text-bottom"><path d="M10.226 17.284c-2.965-.36-5.054-2.493-5.054-5.256 0-1.123.404-2.336 1.078-3.144-.292-.741-.247-2.314.09-2.965.898-.112 2.111.36 2.83 1.01.853-.269 1.752-.404 2.853-.404 1.1 0 1.999.135 2.807.382.696-.629 1.932-1.1 2.83-.988.315.606.36 2.179.067 2.942.72.854 1.101 2 1.101 3.167 0 2.763-2.089 4.852-5.098 5.234.763.494 1.28 1.572 1.28 2.807v2.336c0 .674.561 1.056 1.235.786 4.066-1.55 7.255-5.615 7.255-10.646C23.5 6.188 18.334 1 11.978 1 5.62 1 .5 6.188.5 12.545c0 4.986 3.167 9.12 7.435 10.669.606.225 1.19-.18 1.19-.786V20.63a2.9 2.9 0 0 1-1.078.224c-1.483 0-2.359-.808-2.987-2.313-.247-.607-.517-.966-1.034-1.033-.27-.023-.359-.135-.359-.27 0-.27.45-.471.898-.471.652 0 1.213.404 1.797 1.235.45.651.921.943 1.483.943.561 0 .92-.202 1.437-.719.382-.381.674-.718.944-.943"></path></svg></a><div class="AuthCTAs-module__mobileActions__NNzeV"><a class="Primer_Brand__Button-module__Button___scH9Z Primer_Brand__Button-module__Button--subtle___F7pEE Primer_Brand__Button-module__Button--size-small___zQrEw AuthCTAs-module__cta__WpwQq" href="/login?return_to=https%3A%2F%2Fgithub.com%2Fwalley%2Frustama" data-analytics-event="{&quot;action&quot;:&quot;sign_in&quot;,&quot;tag&quot;:&quot;link&quot;,&quot;context&quot;:&quot;auth_cta&quot;,&quot;location&quot;:&quot;header&quot;,&quot;label&quot;:&quot;sign_in_link_auth_cta_header&quot;}" data-hydro-click="{&quot;event_type&quot;:&quot;authentication.click&quot;,&quot;payload&quot;:{&quot;location_in_page&quot;:&quot;site header menu&quot;,&quot;repository_id&quot;:null,&quot;auth_type&quot;:&quot;SIGN_UP&quot;,&quot;originating_url&quot;:&quot;https://github.com/walley/rustama&quot;,&quot;user_id&quot;:null}}" data-hydro-click-hmac="6ed2305d54d33e9f50a0c05c4a926f9cb86d06d219af895e77af224601d4f14c"><span class="Primer_Brand__Button-module__Button__text___ED0bX"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--100___B2ueX Primer_Brand__Text-module__Text--weight-medium___qJKf_ Primer_Brand__Button-module__Button--label___qrkyz Primer_Brand__Button-module__Button--label-subtle___8ndWH">Sign in</span></span></a><button class="Primer_Brand__Button-module__Button___scH9Z Primer_Brand__Button-module__Button--subtle___F7pEE Primer_Brand__Button-module__Button--size-small___zQrEw HeaderAppearanceSettings-module__trigger__hUheK" type="button" aria-haspopup="dialog" aria-labelledby="_R_3dd_"><span class="Primer_Brand__Button-module__Button__leading-visual___jjtTe" data-testid="Button-leading-visual"><svg data-component="Octicon" focusable="false" aria-hidden="true" class="octicon octicon-sliders Primer_Brand__Button-module__Button__icon-visual____qybb" viewBox="0 0 16 16" width="16" height="16" fill="currentColor" display="inline-block" overflow="visible" style="vertical-align:text-bottom"><path d="M15 2.75a.75.75 0 0 1-.75.75h-4a.75.75 0 0 1 0-1.5h4a.75.75 0 0 1 .75.75Zm-8.5.75v1.25a.75.75 0 0 0 1.5 0v-4a.75.75 0 0 0-1.5 0V2H1.75a.75.75 0 0 0 0 1.5H6.5Zm1.25 5.25a.75.75 0 0 0 0-1.5h-6a.75.75 0 0 0 0 1.5h6ZM15 8a.75.75 0 0 1-.75.75H11.5V10a.75.75 0 1 1-1.5 0V6a.75.75 0 0 1 1.5 0v1.25h2.75A.75.75 0 0 1 15 8Zm-9 5.25v-2a.75.75 0 0 0-1.5 0v1.25H1.75a.75.75 0 0 0 0 1.5H4.5v1.25a.75.75 0 0 0 1.5 0v-2Zm9 0a.75.75 0 0 1-.75.75h-6a.75.75 0 0 1 0-1.5h6a.75.75 0 0 1 .75.75Z"></path></svg></span><span class="Primer_Brand__Button-module__Button__text___ED0bX"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--100___B2ueX Primer_Brand__Text-module__Text--weight-medium___qJKf_ Primer_Brand__Button-module__Button--label___qrkyz Primer_Brand__Button-module__Button--label-subtle___8ndWH"></span></span></button><div class="Primer_Brand__Tooltip-module__Tooltip___0Eipx" data-direction="s" aria-hidden="true" id="_R_3dd_">Appearance settings</div></div></div><div class="MarketingHeader-module__menu__GIy3y"><div class="MarketingHeader-module__menuWrapper__owstH"><nav class="MarketingNavigation-module__nav__W0KYY" aria-label="Global"><ul class="MarketingNavigation-module__list__tFbMb"><li><div class="NavDropdown-module__container__l2YeI"><button type="button" class="NavDropdown-module__button__PEHWX" aria-expanded="false" aria-controls="_R_nd_">Platform<svg data-component="Octicon" aria-hidden="true" focusable="false" class="octicon octicon-triangle-right NavDropdown-module__buttonIcon__Tkl8_" viewBox="0 0 16 16" width="16" height="16" fill="currentColor" display="inline-block" overflow="visible" style="vertical-align:text-bottom"><path d="m6.427 4.427 3.396 3.396a.25.25 0 0 1 0 .354l-3.396 3.396A.25.25 0 0 1 6 11.396V4.604a.25.25 0 0 1 .427-.177Z"></path></svg></button><div id="_R_nd_" class="NavDropdown-module__dropdown__xm1jd"><ul class="NavDropdown-module__list__zuCgG"><li><div class="NavGroup-module__group__W8SqJ"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--monospace___QXHDQ Primer_Brand__Text-module__Text--muted___rE6mh Primer_Brand__Text-module__Text--100___B2ueX Primer_Brand__Text-module__Text--weight-medium___qJKf_ NavGroup-module__title__Wzxz2" id="_R_5knd_">AI CODE CREATION</span><ul class="NavGroup-module__list__UCOFy" aria-labelledby="_R_5knd_"><li><a class="Primer_Brand__Link-module__Link___lF11y Primer_Brand__Link-module__Link--default___VRVW0 NavLink-module__link__EG3d4" href="https://github.com/features/copilot" data-analytics-event="{&quot;action&quot;:&quot;github_copilot&quot;,&quot;tag&quot;:&quot;link&quot;,&quot;context&quot;:&quot;platform&quot;,&quot;location&quot;:&quot;navbar&quot;,&quot;label&quot;:&quot;github_copilot_link_platform_navbar&quot;}"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS Primer_Brand__Link-module__Link--label___jM8Ty"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS Primer_Brand__Text-module__Text--weight-medium___qJKf_ NavLink-module__title__Q7t0p"><svg data-component="Octicon" aria-hidden="true" focusable="false" class="octicon octicon-copilot NavLink-module__icon__ltGNM" viewBox="0 0 16 16" width="16" height="16" fill="currentColor" display="inline-block" overflow="visible" style="vertical-align:text-bottom"><path d="M7.998 15.035c-4.562 0-7.873-2.914-7.998-3.749V9.338c.085-.628.677-1.686 1.588-2.065.013-.07.024-.143.036-.218.029-.183.06-.384.126-.612-.201-.508-.254-1.084-.254-1.656 0-.87.128-1.769.693-2.484.579-.733 1.494-1.124 2.724-1.261 1.206-.134 2.262.034 2.944.765.05.053.096.108.139.165.044-.057.094-.112.143-.165.682-.731 1.738-.899 2.944-.765 1.23.137 2.145.528 2.724 1.261.566.715.693 1.614.693 2.484 0 .572-.053 1.148-.254 1.656.066.228.098.429.126.612.012.076.024.148.037.218.924.385 1.522 1.471 1.591 2.095v1.872c0 .766-3.351 3.795-8.002 3.795Zm0-1.485c2.28 0 4.584-1.11 5.002-1.433V7.862l-.023-.116c-.49.21-1.075.291-1.727.291-1.146 0-2.059-.327-2.71-.991A3.222 3.222 0 0 1 8 6.303a3.24 3.24 0 0 1-.544.743c-.65.664-1.563.991-2.71.991-.652 0-1.236-.081-1.727-.291l-.023.116v4.255c.419.323 2.722 1.433 5.002 1.433ZM6.762 2.83c-.193-.206-.637-.413-1.682-.297-1.019.113-1.479.404-1.713.7-.247.312-.369.789-.369 1.554 0 .793.129 1.171.308 1.371.162.181.519.379 1.442.379.853 0 1.339-.235 1.638-.54.315-.322.527-.827.617-1.553.117-.935-.037-1.395-.241-1.614Zm4.155-.297c-1.044-.116-1.488.091-1.681.297-.204.219-.359.679-.242 1.614.091.726.303 1.231.618 1.553.299.305.784.54 1.638.54.922 0 1.28-.198 1.442-.379.179-.2.308-.578.308-1.371 0-.765-.123-1.242-.37-1.554-.233-.296-.693-.587-1.713-.7Z"></path><path d="M6.25 9.037a.75.75 0 0 1 .75.75v1.501a.75.75 0 0 1-1.5 0V9.787a.75.75 0 0 1 .75-.75Zm4.25.75v1.501a.75.75 0 0 1-1.5 0V9.787a.75.75 0 0 1 1.5 0Z"></path></svg>GitHub Copilot</span><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--muted___rE6mh Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS NavLink-module__subtitle__X4gkW">Write better code with AI</span></span></a></li><li><a class="Primer_Brand__Link-module__Link___lF11y Primer_Brand__Link-module__Link--default___VRVW0 NavLink-module__link__EG3d4" href="https://github.com/features/ai/github-app" data-analytics-event="{&quot;action&quot;:&quot;github_copilot_app&quot;,&quot;tag&quot;:&quot;link&quot;,&quot;context&quot;:&quot;platform&quot;,&quot;location&quot;:&quot;navbar&quot;,&quot;label&quot;:&quot;github_copilot_app_link_platform_navbar&quot;}"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS Primer_Brand__Link-module__Link--label___jM8Ty"><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--default___GhPh_ Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS Primer_Brand__Text-module__Text--weight-medium___qJKf_ NavLink-module__title__Q7t0p"><svg data-component="Octicon" aria-hidden="true" focusable="false" class="octicon octicon-mark-github NavLink-module__icon__ltGNM" viewBox="0 0 16 16" width="16" height="16" fill="currentColor" display="inline-block" overflow="visible" style="vertical-align:text-bottom"><path d="M6.766 11.328c-2.063-.25-3.516-1.734-3.516-3.656 0-.781.281-1.625.75-2.188-.203-.515-.172-1.609.063-2.062.625-.078 1.468.25 1.968.703.594-.187 1.219-.281 1.985-.281.765 0 1.39.094 1.953.265.484-.437 1.344-.765 1.969-.687.218.422.25 1.515.046 2.047.5.593.766 1.39.766 2.203 0 1.922-1.453 3.375-3.547 3.64.531.344.89 1.094.89 1.954v1.625c0 .468.391.734.86.547C13.781 14.359 16 11.53 16 8.03 16 3.61 12.406 0 7.984 0 3.563 0 0 3.61 0 8.031a7.88 7.88 0 0 0 5.172 7.422c.422.156.828-.125.828-.547v-1.25c-.219.094-.5.156-.75.156-1.031 0-1.64-.562-2.078-1.609-.172-.422-.36-.672-.719-.719-.187-.015-.25-.093-.25-.187 0-.188.313-.328.625-.328.453 0 .844.281 1.25.86.313.452.64.655 1.031.655s.641-.14 1-.5c.266-.265.47-.5.657-.656"></path></svg>GitHub Copilot app</span><span class="Primer_Brand__Text-module__Text___XeGJJ Primer_Brand__Text-module__Text-font--mona-sans___a8XJD Primer_Brand__Text-module__Text--muted___rE6mh Primer_Brand__Text-module__Text--200____P1wy Primer_Brand__Text-module__Text--antialiased___TYoXS Na...[truncated, total 390628 bytes]
```

_Thinking:_ Now I have the GitHub repo pages. I can see the meta description tags from each page:

1. **certik**: `content="certik bertik. Contribute to walley/certik development by creating an account on GitHub."`
2. **bertik**: `content="companion to certik. Contribute to walley/bertik development by creating an account on GitHub."`
3. **rustama**: `content="rust tui ollama client with agentic mode, tools, internal terminal - walley/Rustama"`

Also, from the title tags I can see:
- certik: "GitHub - walley/certik: certik bertik"
- bertik: "GitHub - walley/bertik: companion to certik"  
- rustama: "GitHub - walley/Rustama: rust tui ollama client with agentic mode, tools, internal terminal"

Now I need to create a nice HTML page displaying these projects. Let me create a visually appealing page with:
- Project cards or grid layout
- Project names/descriptions
- Links to the GitHub repos
- Pretty styling using CSS

I'll extract the key info and create the HTML file.

**Assistant:** Now I have the information I need from the GitHub pages. Let me extract the key details and create a beautiful HTML page.

From the fetched pages, I can see:
1. **certik**: Description "certik bertik", title "walley/certik: certik bertik"
2. **bertik**: Description "companion to certik", title "walley/bertik: companion to certik"  
3. **rustama**: Description "rust tui ollama client with agentic mode, tools, internal terminal", title "walley/Rustama: rust tui ollama client with agentic mode, tools, internal terminal"

Now let me create a nice HTML page:



**Tool Call:** `write_file`
```
{"content":"<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"UTF-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n  <title>Walley's Projects</title>\n  <link href=\"https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap\" rel=\"stylesheet\">\n  <style>\n    :root {\n      --bg: #0a0a0f;\n      --bg-subtle: #12121a;\n      --fg: #e8e8ec;\n      --muted: #6b6b80;\n      --accent: #00d4aa;\n      --accent-dim: rgba(0, 212, 170, 0.15);\n      --card: #16161f;\n      --border: #2a2a3a;\n      --radius: 12px;\n    }\n\n    * {\n      box-sizing: border-box;\n    }\n\n    body {\n      font-family: 'Inter', sans-serif;\n      background: var(--bg);\n      color: var(--fg);\n      margin: 0;\n      min-height: 100vh;\n      line-height: 1.6;\n    }\n\n    .container {\n      max-width: 1200px;\n      margin: 0 auto;\n      padding: 2rem;\n    }\n\n    h1 {\n      font-weight: 600;\n      font-size: clamp(2rem, 5vw, 3rem);\n      margin-bottom: 0.5rem;\n      background: linear-gradient(135deg, var(--fg), var(--muted));\n      -webkit-background-clip: text;\n      -webkit-text-fill-color: transparent;\n      background-clip: text;\n    }\n\n    .subtitle {\n      color: var(--muted);\n      font-size: 1.1rem;\n      margin-bottom: 3rem;\n    }\n\n    .projects-grid {\n      display: grid;\n      grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));\n      gap: 1.5rem;\n      margin-top: 2rem;\n    }\n\n    .project-card {\n      background: var(--card);\n      border: 1px solid var(--border);\n      border-radius: var(--radius);\n      padding: 1.5rem;\n      transition: all 0.3s ease;\n      cursor: pointer;\n      display: flex;\n      flex-direction: column;\n      height: 100%;\n    }\n\n    .project-card:hover {\n      border-color: var(--accent);\n      transform: translateY(-4px);\n      box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3), 0 0 0 1px var(--accent-dim);\n    }\n\n    .project-card h3 {\n      font-size: 1.25rem;\n      font-weight: 600;\n      margin-bottom: 0.5rem;\n      display: -webkit-box;\n      -webkit-line-clamp: 2;\n      -webkit-box-orient: vertical;\n      overflow: hidden;\n    }\n\n    .project-card p {\n      color: var(--muted);\n      font-size: 0.875rem;\n      flex-grow: 1;\n      margin-bottom: 1rem;\n      display: -webkit-box;\n      -webkit-line-clamp: 3;\n      -webkit-box-orient: vertical;\n      overflow: hidden;\n    }\n\n    .project-link {\n      display: inline-flex;\n      align-items: center;\n      gap: 0.5rem;\n      padding: 0.5rem 1rem;\n      background: var(--accent-dim);\n      color: var(--accent);\n      border-radius: 6px;\n      font-size: 0.875rem;\n      font-weight: 500;\n      transition: background 0.2s;\n      margin-top: auto;\n    }\n\n    .project-link:hover {\n      background: var(--accent);\n      color: var(--bg);\n    }\n\n    .project-link svg {\n      width: 14px;\n      height: 14px;\n    }\n\n    .repo-link {\n      color: var(--accent);\n      text-decoration: none;\n      font-weight: 500;\n      transition: opacity 0.2s;\n    }\n\n    .repo-link:hover {\n      opacity: 0.8;\n    }\n\n    .view-all {\n      color: var(--muted);\n      text-decoration: none;\n      font-size: 0.875rem;\n      transition: color 0.2s;\n    }\n\n    .view-all:hover {\n      color: var(--accent);\n    }\n\n    @media (max-width: 640px) {\n      .container {\n        padding: 1rem;\n      }\n\n      h1 {\n        font-size: 1.75rem;\n      }\n\n      .project-card {\n        padding: 1.25rem;\n      }\n    }\n  </style>\n</head>\n<body>\n  <div class=\"container\">\n    <h1>Walley's Projects</h1>\n    <p class=\"subtitle\">A collection of my GitHub projects</p>\n\n    <div class=\"projects-grid\">\n      <!-- certik -->\n      <article class=\"project-card\">\n        <h3>certik</h3>\n        <p>certik bertik - A foundational project by walley</p>\n        <a href=\"https://github.com/walley/certik\" class=\"repo-link\" target=\"_blank\" rel=\"noopener\">\n          walley/certik\n        </a>\n        <a href=\"https://github.com/walley/certik\" class=\"project-link\" target=\"_blank\" rel=\"noopener\">\n          <svg viewBox=\"0 0 24 24\" fill=\"currentColor\">\n            <path d=\"M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.575v2.06c0 .807-.371 1.637-.99 2.386.621.373.998.615.998.615l2.187-1.268c-.168-.531-.193-.989-.088-1.433.117-.475.548-.768 1.085-.768 1.058 0 1.968.433 2.387 1.096.126-.212.378-.535.566-.855-.344.552-1.05.937-1.357 1.234-.41.405-.838.714-1.323.896-.596.152-1.235.173-1.853.137-.693-.365-1.071-.913-1.071-1.575v-2.133c-.03-.318-.154-.596-.38-.852-.377-.378-.988-.578-1.675-.578-.897 0-1.623.348-2.117.93-.064.079-.163.128-.23.128-.199 0-.366-.077-.488-.236-.176-.228-.307-.418-.357-.603-.055-.237-.017-.416.138-.568.231-.202.55-.357.919-.357 1.004 0 1.813.4 2.338 1.088.177.218.428.37.732.395.373.03 1.06.073 1.79.073 1.215 0 2.183-.31 2.728-.948.396-.446.573-1.068.573-1.768v-2.253c0-1.118-.422-1.797-1.265-2.28-.31-.177-.708-.238-1.218-.238-.794 0-1.382.336-1.782.966-.517.715-1.278.978-2.185.978-1.493 0-2.625-.86-2.625-1.936 0-1.318.805-2.357 2.227-2.357.588 0 1.17.07 1.733.21.367.067.653.133.946.163.697.036 1.33.036 1.964 0 1.075-.28 1.997-.853 2.787-.39.405-.862.655-1.37.655-.198 0-.383-.023-.566-.073zM12 1c6.627 0 12 5.372 12 12s-5.373 12-12 12-12-5.372-12-12 5.373-12 12-12z\"/>\n          </svg>\n          View on GitHub\n        </a>\n      </article>\n\n      <!-- bertik -->\n      <article class=\"project-card\">\n        <h3>bertik</h3>\n        <p>companion to certik - walley's follow-up project</p>\n        <a href=\"https://github.com/walley/bertik\" class=\"repo-link\" target=\"_blank\" rel=\"noopener\">\n          walley/bertik\n        </a>\n        <a href=\"https://github.com/walley/bertik\" class=\"project-link\" target=\"_blank\" rel=\"noopener\">\n          <svg viewBox=\"0 0 24 24\" fill=\"currentColor\">\n            <path d=\"M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.575v2.06c0 .807-.371 1.637-.99 2.386.621.373.998.615.998.615l2.187-1.268c-.168-.531-.193-.989-.088-1.433.117-.475.548-.768 1.085-.768 1.058 0 1.968.433 2.387 1.096.126-.212.378-.535.566-.855-.344.552-1.05.937-1.357 1.234-.41.405-.838.714-1.323.896-.596.152-1.235.173-1.853.137-.693-.365-1.071-.913-1.071-1.575v-2.133c-.03-.318-.154-.596-.38-.852-.377-.378-.988-.578-1.675-.578-.897 0-1.623.348-2.117.93-.064.079-.163.128-.23.128-.199 0-.366-.077-.488-.236-.176-.228-.307-.418-.357-.603-.055-.237-.017-.416.138-.568.231-.202.55-.357.919-.357 1.004 0 1.813.4 2.338 1.088.177.218.428.37.732.395.373.03 1.06.073 1.79.073 1.215 0 2.183-.31 2.728-.948.396-.446.573-1.068.573-1.768v-2.253c0-1.118-.422-1.797-1.265-2.28-.31-.177-.708-.238-1.218-.238-.794 0-1.382.336-1.782.966-.517.715-1.278.978-2.185.978-1.493 0-2.625-.86-2.625-1.936 0-1.318.805-2.357 2.227-2.357.588 0 1.17.07 1.733.21.367.067.653.133.946.163.697.036 1.33.036 1.964 0 1.075-.28 1.997-.853 2.787-.39.405-.862.655-1.37.655-.198 0-.383-.023-.566-.073zM12 1c6.627 0 12 5.372 12 12s-5.373 12-12 12-12-5.372-12-12 5.373-12 12-12z\"/>\n          </svg>\n          View on GitHub\n        </a>\n      </article>\n\n      <!-- rustama -->\n      <article class=\"project-card\">\n        <h3>rustama</h3>\n        <p>rust tui ollama client with agentic mode, tools, internal terminal</p>\n        <a href=\"https://github.com/walley/rustama\" class=\"repo-link\" target=\"_blank\" rel=\"noopener\">\n          walley/Rustama\n        </a>\n        <a href=\"https://github.com/walley/rustama\" class=\"project-link\" target=\"_blank\" rel=\"noopener\">\n          <svg viewBox=\"0 0 24 24\" fill=\"currentColor\">\n            <path d=\"M12 0c-6.627 0-12 5.373-12 12s5.373 12 12 12 12-5.373 12-12-5.373-12-12-12zm0-2c-3.866 0-7 3.134-7 7v2.069c0 .518.168.873.5 1.195l3.365-.389c-.178.012-.356.023-.535.023-.548 0-.938-.44-.938-.985v-2.155c0-.508.26-.83.66-.983l3.365-.375c.028-.003.056-.006.084-.009l3.625 5.082c.038.053.077.098.117.136.053.045.087.078.098.113l3.093 13.882c-.016-.456-.12-.862-.318-1.233-.134-.278-.318-.48-.566-.653-.338-.225-.67-.397-.989-.526l3.262-5.568c.038-.053.057-.098.066-.136l2.958-4.855c.003-.113.017-.212.03-.295.037-.156.087-.31.137-.436.076-.166.13-.37.165-.552.04-.237.053-.456.04-.673l-3.632 5.563c-.528.048-.98.12-.137.673l3.629-5.566c.053-.045.098-.089.124-.128l3.303-4.835c.038-.053.057-.098.057-.136l-3.093-13.882c.044-.035.088-.07.122-.105.034-.035.058-.07.07-.105.012-.035.018-.07.023-.105l3.365.389c-.056.548-.446.938-.938.938v2.155zM9 18c-1.657 0-3-1.343-3-3s1.343-3 3-3 3 1.343 3 3-1.343 3-3 3zm3-6c.552 0 1-.448 1-1s-.448-1-1-1-1 .448-1 1 .448 1 1 1z\"/>\n          </svg>\n          View on GitHub\n        </a>\n      </article>\n    </div>\n  </div>\n</body>\n</html>","path":"/workspace/projects.html"}
```

**Tool Result:** `write_file`
```
Error writing file: No such file or directory (os error 2)
```

_Thinking:_ The directory doesn't exist. Let me check what directories are available and create the file in an appropriate location.

**Assistant:** 


**Tool Call:** `bash`
```
{"command":"mkdir -p /workspace && ls /workspace"}
```

**Tool Result:** `bash`
```
mkdir: cannot create directory ‘/workspace’: Permission denied

```

_Thinking:_ Let me try creating the file in the current working directory instead.

**Assistant:** 


**Tool Call:** `bash`
```
{"command":"pwd && ls"}
```

**Tool Result:** `bash`
```
/home/walley/git/Rustama
AGENTS.md
Cargo.lock
Cargo.toml
doc
LICENSE
plan
README.md
rustama.log
rustama.log.1
rustama.log.2
rustama.log.3
rustama.log.4
rustama.log.5
rustama.log.6
rustama.log.7
rustama.log.8
rustama.log.9
rustama-lsp.log
src
summary.md
target
UI.md

```

_Thinking:_ I'm in /home/walley/git/Rustama directory. Let me create the HTML file here.

⚠ Stream ended unexpectedly (partial response shown)

**User:** continue

_Thinking:_ Now let me create the HTML file in the current directory.

Error: Empty response from model

**User:** > continue

_Thinking:_ The user wants me to continue creating the HTML file. Let me write it to the current

Error: Empty response from model

**User:** type html here, i will copy it where i need it

_Thinking:_ The user wants me to type the HTML content so they can copy it where they need it. I already have the HTML prepared from my previous attempt. Let me output just the HTML content without the file write operation.

**Assistant:** Here's the HTML code for your projects page:

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Walley's Projects</title>
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap" rel="stylesheet">
  <style>
    :root {
      --bg: #0a0a0f;
      --bg-subtle: #12121a;
      --fg: #e8e8ec;
      --muted: #6b6b80;
      --accent: #00d4aa;
      --accent-dim: rgba(0, 212, 170, 0.15);
      --card: #16161f;
      --border: #2a2a3a;
      --radius: 12px;
    }

    * {
      box-sizing: border-box;
    }

    body {
      font-family: 'Inter', sans-serif;
      background: var(--bg);
      color: var(--fg);
      margin: 0;
      min-height: 100vh;
      line-height: 1.6;
    }

    .container {
      max-width: 1200px;
      margin: 0 auto;
      padding: 2rem;
    }

    h1 {
      font-weight: 600;
      font-size: clamp(2rem, 5vw, 3rem);
      margin-bottom: 0.5rem;
      background: linear-gradient(135deg, var(--fg), var(--muted));
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
    }

    .subtitle {
      color: var(--muted);
      font-size: 1.1rem;
      margin-bottom: 3rem;
    }

    .projects-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
      gap: 1.5rem;
      margin-top: 2rem;
    }

    .project-card {
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: var(--radius);
      padding: 1.5rem;
      transition: all 0.3s ease;
      cursor: pointer;
      display: flex;
      flex-direction: column;
      height: 100%;
    }

    .project-card:hover {
      border-color: var(--accent);
      transform: translateY(-4px);
      box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3), 0 0 0 1px var(--accent-dim);
    }

    .project-card h3 {
      font-size: 1.25rem;
      font-weight: 600;
      margin-bottom: 0.5rem;
      display: -webkit-box;
      -webkit-line-clamp: 2;
      -webkit-box-orient: vertical;
      overflow: hidden;
    }

    .project-card p {
      color: var(--muted);
      font-size: 0.875rem;
      flex-grow: 1;
      margin-bottom: 1rem;
      display: -webkit-box;
      -webkit-line-clamp: 3;
      -webkit-box-orient: vertical;
      overflow: hidden;
    }

    .project-link {
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0.5rem 1rem;
      background: var(--accent-dim);
      color: var(--accent);
      border-radius: 6px;
      font-size: 0.875rem;
      font-weight: 500;
      transition: background 0.2s;
      margin-top: auto;
    }

    .project-link:hover {
      background: var(--accent);
      color: var(--bg);
    }

    .project-link svg {
      width: 14px;
      height: 14px;
    }

    .repo-link {
      color: var(--accent);
      text-decoration: none;
      font-weight: 500;
      transition: opacity 0.2s;
    }

    .repo-link:hover {
      opacity: 0.8;
    }

    .view-all {
      color: var(--muted);
      text-decoration: none;
      font-size: 0.875rem;
      transition: color 0.2s;
    }

    .view-all:hover {
      color: var(--accent);
    }

    @media (max-width: 640px) {
      .container {
        padding: 1rem;
      }

      h1 {
        font-size: 1.75rem;
      }

      .project-card {
        padding: 1.25rem;
      }
    }
  </style>
</head>
<body>
  <div class="container">
    <h1>Walley's Projects</h1>
    <p class="subtitle">A collection of my GitHub projects</p>

    <div class="projects-grid">
      <!-- certik -->
      <article class="project-card">
        <h3>certik</h3>
        <p>certik bertik - A foundational project by walley</p>
        <a href="https://github.com/walley/certik" class="repo-link" target="_blank" rel="noopener">
          walley/certik
        </a>
        <a href="https://github.com/walley/certik" class="project-link" target="_blank" rel="noopener">
          <svg viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.575v2.06c0 .807-.371 1.637-.99 2.386.621.373.998.615.998.615l2.187-1.268c-.168-.531-.193-.989-.088-1.433.117-.475.548-.768 1.085-.768 1.058 0 1.968.433 2.387 1.096.126-.212.378-.535.566-.855-.344.552-1.05.937-1.357 1.234-.41.405-.838.714-1.323.896-.596.152-1.235.173-1.853.137-.693-.365-1.071-.913-1.071-1.575v-2.133c-.03-.318-.154-.596-.38-.852-.377-.378-.988-.578-1.675-.578-.897 0-1.623.348-2.117.93-.064.079-.163.128-.23.128-.199 0-.366-.077-.488-.236-.176-.228-.307-.418-.357-.603-.055-.237-.017-.416.138-.568.231-.202.55-.357.919-.357 1.004 0 1.813.4 2.338 1.088.177.218.428.37.732.395.373.03 1.06.073 1.79.073 1.215 0 2.183-.31 2.728-.948.396-.446.573-1.068.573-1.768v-2.253c0-1.118-.422-1.797-1.265-2.28-.31-.177-.708-.238-1.218-.238-.794 0-1.382.336-1.782.966-.517.715-1.278.978-2.185.978-1.493 0-2.625-.86-2.625-1.936 0-1.318.805-2.357 2.227-2.357.588 0 1.17.07 1.733.21.367.067.653.133.946.163.697.036 1.33.036 1.964 0 1.075-.28 1.997-.853 2.787-.39.405-.862.655-1.37.655-.198 0-.383-.023-.566-.073zM12 1c6.627 0 12 5.372 12 12s-5.373 12-12 12-12-5.372-12-12 5.373-12 12-12z"/>
          </svg>
          View on GitHub
        </a>
      </article>

      <!-- bertik -->
      <article class="project-card">
        <h3>bertik</h3>
        <p>companion to certik - walley's follow-up project</p>
        <a href="https://github.com/walley/bertik" class="repo-link" target="_blank" rel="noopener">
          walley/bertik
        </a>
        <a href="https://github.com/walley/bertik" class="project-link" target="_blank" rel="noopener">
          <svg viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.575v2.06c0 .807-.371 1.637-.99 2.386.621.373.998.615.998.615l2.187-1.268c-.168-.531-.193-.989-.088-1.433.117-.475.548-.768 1.085-.768 1.058 0 1.968.433 2.387 1.096.126-.212.378-.535.566-.855-.344.552-1.05.937-1.357 1.234-.41.405-.838.714-1.323.896-.596.152-1.235.173-1.853.137-.693-.365-1.071-.913-1.071-1.575v-2.133c-.03-.318-.154-.596-.38-.852-.377-.378-.988-.578-1.675-.578-.897 0-1.623.348-2.117.93-.064.079-.163.128-.23.128-.199 0-.366-.077-.488-.236-.176-.228-.307-.418-.357-.603-.055-.237-.017-.416.138-.568.231-.202.55-.357.919-.357 1.004 0 1.813.4 2.338 1.088.177.218.428.37.732.395.373.03 1.06.073 1.79.073 1.215 0 2.183-.31 2.728-.948.396-.446.573-1.068.573-1.768v-2.253c0-1.118-.422-1.797-1.265-2.28-.31-.177-.708-.238-1.218-.238-.794 0-1.382.336-1.782.966-.517.715-1.278.978-2.185.978-1.493 0-2.625-.86-2.625-1.936 0-1.318.805-2.357 2.227-2.357.588 0 1.17.07 1.733.21.367.067.653.133.946.163.697.036 1.33.036 1.964 0 1.075-.28 1.997-.853 2.787-.39.405-.862.655-1.37.655-.198 0-.383-.023-.566-.073zM12 1c6.627 0 12 5.372 12 12s-5.373 12-12 12-12-5.372-12-12 5.373-12 12-12z"/>
          </svg>
          View on GitHub
        </a>
      </article>

      <!-- rustama -->
      <article class="project-card">
        <h3>rustama</h3>
        <p>rust tui ollama client with agentic mode, tools, internal terminal</p>
        <a href="https://github.com/walley/rustama" class="repo-link" target="_blank" rel="noopener">
          walley/Rustama
        </a>
        <a href="https://github.com/walley/rustama" class="project-link" target="_blank" rel="noopener">
          <svg viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 0c-6.627 0-12 5.373-12 12s5.373 12 12 12 12-5.373 12-12-5.373-12-12-12zm0-2c-3.866 0-7 3.134-7 7v2.069c0 .518.168.873.5 1.195l3.365-.389c-.178.012-.356.023-.535.023-.548 0-.938-.44-.938-.985v-2.155c0-.508.26-.83.66-.983l3.365-.375c.028-.003.056-.006.084-.009l3.625 5.082c.038.053.077.098.117.136.053.045.087.078.098.113l3.093 13.882c-.016-.456-.12-.862-.318-1.233-.134-.278-.318-.48-.566-.653-.338-.225-.67-.397-.989-.526l3.262-5.568c.038-.053.057-.098.066-.136l2.958-4.855c.003-.113.017-.212.03-.295.037-.156.087-.31.137-.436.076-.166.13-.37.165-.552.04-.237.053-.456.04-.673l-3.632 5.563c-.528.048-.98.12-.137.673l3.629-5.566c.053-.045.098-.089.124-.128l3.303-4.835c.038-.053.057-.098.057-.136l-3.093-13.882c.044-.035.088-.07.122-.105.034-.035.058-.07.07-.105.012-.035.018-.07.023-.105l3.365.389c-.056.548-.446.938-.938.938v2.155zM9 18c-1.657 0-3-1.343-3-3s1.343-3 3-3 3 1.343 3 3-1.343 3-3 3zm3-6c.552 0 1-.448 1-1s-.448-1-1-1-1 .448-1 1 .448 1 1 1z"/>
          </svg>
          View on GitHub
        </a>
      </article>
    </div>
  </div>
</body>
</html>
```

