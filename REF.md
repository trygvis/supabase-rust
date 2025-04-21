Supabase v2.x パッケージ主要API一覧

1. @supabase/supabase-js
	•	createClient(supabaseUrl, supabaseKey, options?) – SupabaseプロジェクトのURLと公開APIキーからSupabaseクライアントを生成する関数。optionsでは認証やスキーマ、通信設定を指定可能。戻り値はSupabaseClientインスタンスです。
	•	SupabaseClient クラス – createClientが返すメインのクライアントクラスで、各種サービスへの操作メソッドを提供します。
	•	from(table: string) – 指定したテーブル名に対するデータ操作クエリを開始します。戻り値としてそのテーブルに対するクエリビルダ（PostgrestQueryBuilder）を返し、select, insert, update, deleteなどの連鎖的操作を行えます ￼。
	•	rpc(fn: string, params?: object, options?: { head?: boolean, count?: 'exact'|'planned'|'estimated' }) – Postgresのストアドプロシージャ（関数）をRPC経由で呼び出します。paramsで関数への引数を指定します。戻り値はクエリビルダ（PostgrestFilterBuilder）で、selectと同様に結果にフィルタや取得方法を連鎖的に指定できます ￼。
	•	auth プロパティ – 認証機能のクライアント（SupabaseAuthClient）です。ユーザーのサインアップやログイン、セッション管理などのメソッドを利用できます（詳細は**@supabase/auth-js**を参照）。
	•	storage プロパティ – ストレージ機能のクライアント（StorageClient）です。ストレージバケットやファイルの操作メソッド（後述の**@supabase/storage-js**参照）を利用できます。
	•	functions プロパティ – エッジ関数を呼び出すためのクライアント（FunctionsClient）です。.invoke()メソッドでサーバレス関数を実行できます（詳細は**@supabase/functions-js**を参照）。
	•	channel(name: string, opts?: object) – リアルタイムチャンネルを作成します。nameでチャンネル名を指定し、オプションでBroadcastやPresence機能の設定を渡せます。戻り値はRealtimeChannelオブジェクトで、後述のメソッド（.on(), .subscribe()など）でリアルタイム購読を制御します ￼ ￼。
	•	getChannels() – 現在クライアントで接続している全てのRealtimeチャンネルを取得します。戻り値はRealtimeChannelオブジェクトの配列です ￼。
	•	removeChannel(channel: RealtimeChannel) – 指定したリアルタイムチャンネルを購読解除し、クライアントから削除します。戻り値は'ok'・'timed out'・'error'のいずれかのステータスを解決するPromiseです ￼。
	•	removeAllChannels() – 全てのリアルタイムチャンネルを購読解除して削除します。戻り値は各チャンネルの解除ステータス文字列を要素とするPromiseです ￼。
	•	schema(schemaName: string) – 指定したスキーマに対するPostgRESTクライアントを取得します。デフォルトはpublicスキーマですが、このメソッドで得たクライアントを使うと他のスキーマのテーブルに対してfrom()やrpc()を呼ぶことができます。

2. @supabase/auth-js
	•	AuthClient クラス – Supabaseの認証サービスを利用するクライアントクラスです。ユーザー認証に関する様々なメソッドを提供します。AuthClientのインスタンス生成時に{ url: '<auth_endpoint>' }（GoTrueサーバのURL）等をオプション指定できます ￼。
	•	signUp(credentials: { email?: string; password?: string; phone?: string; ... }, options?: { data?: object; ... }) – 新規ユーザーをメールアドレス・電話番号とパスワードで登録します。emailまたはphoneとpasswordを含むオブジェクトを渡し、必要に応じてユーザメタデータ（data）等を付加できます。登録後、セッション情報（ユーザーとトークン）を含むレスポンスを返します。
	•	signInWithPassword(credentials: { email?: string; phone?: string; password: string }) – メールアドレスまたは電話番号とパスワードでユーザーをサインイン（ログイン）させます。既存ユーザーの認証に使用し、成功時にはセッション情報を返します ￼。
	•	signInWithOtp(credentials: { email?: string; phone?: string; options?: { emailRedirectTo?: string; shouldCreateUser?: boolean; ... } }) – パスワードなしでのサインイン（いわゆるマジックリンクやワンタイムパスコードによるログイン）を行います。メールアドレスまたは電話番号を渡すと、ユーザーにワンタイムリンクまたはコードを送信します ￼。既存ユーザーがいない場合はオプションで自動サインアップも可能です。
	•	signInWithOAuth(credentials: { provider: 'google'|'github'|...; options?: { redirectTo?: string; scopes?: string; ... } }) – GoogleやGitHubなどのOAuthプロバイダーでサインインを開始します。providerにプロバイダー名を指定し、このメソッドを呼ぶとOAuth認可フローが開始されます（リダイレクトURLやスコープはoptionsで指定）。戻り値としてOAuth認証用のURLやセッションを含むオブジェクトを返します。
	•	signOut() – 現在のユーザーセッションをログアウトします。ローカルに保持したトークンを削除し、サーバ側でもリフレッシュトークンを無効化します。成功時はエラーがnullとなるシンプルな結果を返します ￼。
	•	updateUser(attributes: { password?: string; email?: string; phone?: string; data?: object; ... }) – ログイン中のユーザー情報を更新します。パスワード変更やメール・電話の変更、あるいはユーザープロフィール情報（メタデータ）の更新に使用できます。変更できる項目はオプションで指定し、更新後のユーザーデータを返します。
	•	getSession() – 現在のセッション情報を取得します。ログイン中であればアクセストークンやユーザー情報を含むセッションを返し、未ログインであればnullを返します。セッションが有効期限切れの場合、自動でトークンリフレッシュを試みます。
	•	getUser() – 現在ログインしているユーザーの情報を取得します。ユーザーIDやメールアドレス、ユーザーメタデータなどを含むオブジェクトを返します。セッションが存在しない場合はnullとなります。
	•	resetPasswordForEmail(email: string, options?: { redirectTo?: string }) – 指定したメールアドレス宛にパスワードリセット用のメールを送信します。成功すると、ユーザーは受け取ったメールのリンクから新しいパスワードを設定できます（redirectToでリンク後に遷移するURLを指定可能） ￼。戻り値は送信結果のエラー情報などです。
	•	onAuthStateChange(callback: (event: 'SIGNED_IN'|'SIGNED_OUT'|'TOKEN_REFRESHED'|'USER_UPDATED'|'PASSWORD_RECOVERY'|'MFA_CHALLENGE', session: Session|null) => void) – 認証状態の変化イベントを購読するリスナーを登録します。ユーザーのサインイン/サインアウト、トークンの自動更新、ユーザー情報更新、パスワードリカバリ開始などのイベント時にcallbackが呼ばれます。戻り値として解除用のSubscriptionオブジェクトを返します。
	•	setSession(refreshToken: string|{ access_token: string; refresh_token: string }) – リフレッシュトークン（またはアクセストークンとセット）から現在のセッションを設定します。サーバーに問い合わせて新しいアクセストークンを発行し、セッションを復元します ￼。主にSSR（サーバサイドレンダリング）やカスタム認証フローで、既存のトークンからクライアント状態を初期化する際に使います。
	•	管理者用メソッド (auth.admin): サーバーサイドでサービスキーを用いた管理操作も提供されています。例えば、auth.admin.createUser()で任意のメールアドレスにユーザーを作成したり、auth.admin.deleteUser(uid)でユーザーを削除したりできます ￼。他にもユーザー一覧の取得や、メール確認リンク・招待メールの送信、ユーザ情報の更新等の管理APIを備えています。

3. @supabase/functions-js
	•	FunctionsClient クラス – Supabaseのエッジ関数を呼び出すためのクライアントです。FunctionsClientのインスタンスはsupabase.functionsプロパティ経由や直接生成して利用します。主に次のメソッドを提供します:
	•	invoke(functionName: string, options: { headers?: Record<string, string>; body?: any; redirectTo?: string; ... }) – サーバレスエッジ関数を呼び出します。functionNameには実行したい関数のデプロイ名を指定し、options.bodyに関数に渡すペイロードを設定します。関数はバックエンドで実行され、その結果が{ data, error }形式で返されます ￼。headersでカスタムヘッダを付与でき、認証が必要な関数の場合は自動的に適切な認証ヘッダが付与されます。レスポンスdataには関数からの返り値が、errorには発生したエラー（無い場合はnull）が含まれます。
	•	エラークラス: 関数呼び出し時のエラーを表すいくつかのエラー型がエクスポートされています。FunctionsHttpErrorは関数がHTTPエラーレスポンス（ステータスコードが200番台以外）を返した場合のエラーで、レスポンスオブジェクトをcontextプロパティに含みます ￼。FunctionsRelayErrorはエッジ関数のリレーサーバー側で発生したエラー、FunctionsFetchErrorはネットワーク障害など関数自体を呼び出せなかった場合のエラーです。これらはinstanceofでチェックでき、エラーごとに適切な処理を行うことが可能です ￼。

4. @supabase/postgrest-js
	•	PostgrestClient クラス – SupabaseのデータベースREST API(PostgREST)に対するクライアントです。データベースのテーブルやビュー、RPCを操作するためのエントリポイントとなります。コンストラクタにPostgRESTエンドポイントのURLとオプション（ヘッダやスキーマ名）を渡して生成します。主なメソッド:
	•	from(table: string) – 指定テーブル名に対するクエリビルダ（PostgrestQueryBuilder）を返します。このビルダでselectやinsertなどの操作を連鎖的に呼び出すことで、テーブルに対するCRUDクエリを構築・実行できます ￼。
	•	rpc(fn: string, params: object, options?: { count?: 'exact'|'planned'|'estimated' }) – データベースのストアドプロシージャ（関数）を呼び出します。内部的には指定関数名に対してPOST /rpc/<fn>リクエストを送ります。paramsで関数の引数をオブジェクト形式で指定します。options.countは結果件数のカウント方法を指定でき、戻り値として結果に対するクエリビルダ（PostgrestFilterBuilder）を返します ￼。このビルダでさらにフィルタやシングル行取得指定などが可能です。
	•	auth(token: string) – 後続のリクエストに認証トークン（JWT）を使用するよう設定します。例えばサービスキーやユーザーJWTをここで設定することで、その後のfromやrpc呼び出しに認証ヘッダが付与されます。メソッド自身はクライアント自身（this）を返すのでチェーン可能です。
	•	PostgrestQueryBuilder クラス – from()やPostgrestClientのメソッドから返されるクエリビルダで、指定したテーブル/ビューに対する各種データ操作メソッドを持ちます。これらのメソッドを呼ぶことでクエリが構築され、最終的にPromiseをawaitすることでリクエストが実行されます。
	•	select(columns?: string, options?: { head?: boolean; count?: 'exact'|'planned'|'estimated' }) – テーブルからデータを取得します。columnsには取得したいカラム名をカンマ区切りで指定（'*'指定で全列）。head:trueの場合データは返さずにレスポンスヘッダのみ取得します。countオプションでカウント方法（正確・推定など）を指定可能です。呼び出すとクエリにフィルタ等を追加できるPostgrestFilterBuilderを返します ￼（デフォルトではさらに.then()で結果を取得するために使います）。
	•	insert(values: object | object[], options?: { upsert?: boolean; onConflict?: string; returning?: 'minimal'|'representation'; count?: 'exact'|'planned'|'estimated' }) – 新規行を挿入します。valuesには挿入するデータオブジェクト、またはオブジェクトの配列を指定します。upsert:trueを指定すると既存重複時は更新に切り替わります（onConflictで一意制約カラムを指定可能）。returningで挿入後に返すデータを'minimal'（何も返さない）か'representation'（挿入後レコードを返す）か選択できます（デフォルトはrepresentation）。実行するとPostgrestFilterBuilderを返し、必要に応じて追加のフィルタや.single()指定が可能です ￼。
	•	upsert(values: object | object[], options?: { onConflict?: string; returning?: 'minimal'|'representation'; count?: ...; ignoreDuplicates?: boolean }) – UPSERT操作（存在すれば更新、なければ挿入）を行います。基本的にinsertの変種で、ignoreDuplicates:trueで重複時に何もしない設定や、onConflictで重複判定に使うカラムを指定できます ￼。戻り値やオプションの意味はinsertと概ね同様です。
	•	update(values: object, options?: { returning?: 'minimal'|'representation'; count?: ... }) – 条件にマッチする行を更新します。更新内容をvaluesのオブジェクトで指定します。returningやcountは上記と同様で、実行時にPostgrestFilterBuilderを返します（更新対象の絞り込みにフィルタが必要なため通常は先に.eq()等で条件指定します）。
	•	delete(options?: { returning?: 'minimal'|'representation'; count?: ... }) – 条件にマッチする行を削除します。returningやcountオプションを指定でき、実行するとPostgrestFilterBuilderを返します（削除する行の条件を指定するため、通常実行前にフィルタメソッドで条件を絞ります）。
	•	PostgrestFilterBuilder クラス – selectやinsertなどデータ操作メソッドの呼び出し後に取得できるビルダで、結果に対するフィルタや変形、制限を指定するメソッドを持ちます。以下主要なメソッド:
	•	フィルタ条件指定:
	•	eq(column: string, value: any) – 指定カラムの値がvalueと等しい行に絞り込みます ￼。
	•	neq(column: string, value: any) – 指定カラムの値がvalueと等しくない行に絞り込みます ￼。
	•	gt(column: string, value: any), gte(column, value) – 指定カラムの値がvalueより大きい（gteは以上）行に絞り込みます ￼。
	•	lt(column: string, value: any), lte(column, value) – 指定カラムの値がvalueより小さい（lteは以下）行に絞り込みます ￼。
	•	like(column: string, pattern: string), ilike(column, pattern) – 指定カラムのテキストがpatternにマッチする行に絞り込みます（ilikeは大文字小文字を区別しない） ￼。
	•	is(column: string, value: boolean | null) – 指定カラムの値がtrue, falseもしくはNULLである行に絞ります ￼。
	•	in(column: string, values: any[]) – 指定カラムの値が配列valuesのいずれかに含まれる行に絞り込みます ￼。
	•	not(column: string, operator: string, value: any) – 指定カラムに対し、与えた演算子と値で否定条件のフィルタを適用します（例: .not('status', 'eq', 'SUCCESS')はstatus != 'SUCCESS'に相当） ￼。
	•	or(filters: string, options?: { foreignTable?: string }) – 複数条件をORで結合します。filtersは例として"a.eq.1,or(b.eq.2,c.eq.3)"のようにカンマで区切ったPostgREST形式の条件文字列を指定します ￼。foreignTableを指定すると外部テーブルに対するOR条件を構築できます。
	•	その他高度なフィルタ: JSON/配列カラムに対する包含演算（contains / containedBy）、範囲型に対する演算子（overlaps, lte, gteなど）、全文検索（textSearch）など多彩なメソッドがあります。それぞれPostgRESTのクエリパラメータに対応しており、例えば.contains('tags', ['a'])は指定配列を含む行、.textSearch('body', 'foo', { type: 'plain' })は全文検索などを表します。
	•	並び替えとページング:
	•	order(column: string, options?: { ascending?: boolean; nullsFirst?: boolean; foreignTable?: string }) – 結果を指定カラムでソートします。ascendingがfalseで降順、nullsFirstでNULL値を先頭にするかを制御できます ￼。
	•	limit(count: number, options?: { foreignTable?: string }) – 取得する行数の上限を指定します ￼。
	•	range(from: number, to: number, options?: { foreignTable?: string }) – 取得する行のインデックス範囲を指定します（0始まりのfromからtoまでを取得） ￼。内部的にはlimitとoffsetを設定します。
	•	単一行取得:
	•	single() – クエリ結果が単一行であることを期待し、その行を直接返すようにします。複数行が返った場合エラーになります。
	•	maybeSingle() – クエリ結果が0または1行であることを期待し、0行の場合はdataにnullを返すようにします。複数行返った場合はエラーとなります。
	•	その他:
	•	throwOnError() – このメソッドをチェーンすると、最終的にthen()で受け取るレスポンスにエラーが含まれていた場合にPromiseをrejectし、例外として投げます ￼。エラーハンドリングをtry/catchで行いたい場合に有用です。
	•	then(...) – ビルダはPromiseライクでもあり、awaitまたは.then()で実行結果を取得できます。thenを呼ぶと内部でHTTPリクエストが実行され、レスポンスとして{ data, error, count, status, statusText }形式のオブジェクトが返ります ￼ ￼。dataに結果データ（selectの場合配列、single()適用時はオブジェクト）、errorにエラー情報、countに件数（count指定時）、statusにHTTPステータスが含まれます。
	•	PostgrestError – PostgRESTから返されるエラーを表すオブジェクト/クラスです。message, details, hint, codeなどのプロパティを持ちます。supabase-jsではクエリ結果errorとしてこのエラー情報が返ります。@supabase/postgrest-jsはPostgrestError型をエクスポートしているので、型情報として利用できます ￼。

5. @supabase/realtime-js
	•	RealtimeClient クラス – SupabaseのRealtimeサーバ（WebSocket）に接続し、リアルタイム機能（Broadcast・Presence・Postgres CDC）を利用するためのクライアントです。コンストラクタにRealtimeエンドポイントのURL（例: 'wss://<project>.supabase.co/realtime/v1'）とオプション（paramsにAPIキーJWTなど）を渡してインスタンス化します ￼。主要メソッド:
	•	channel(name: string, options?: { config?: { broadcast?: { self?: boolean; ack?: boolean }; presence?: { key?: string }; ... } }) – 新しいリアルタイムチャネルを初期化します。nameにはチャンネル名を指定します。optionsでBroadcastやPresenceの挙動設定が可能です（例えばbroadcast.self:trueで自分の送信したメッセージを自分にも配信、presence.keyで現在ユーザーを識別するキーを指定など） ￼ ￼。戻り値はRealtimeChannelオブジェクトです。
	•	getChannels() – 現在クライアントで生成したすべてのチャネルを配列で取得します ￼。各要素はRealtimeChannelインスタンスです。
	•	removeChannel(channel: RealtimeChannel) – 指定したチャネルを購読解除し、クライアントから削除します。例えばclient.removeChannel(channel)のように使い、切断結果（正常終了・タイムアウト・エラー）を示すステータスを返します ￼。
	•	removeAllChannels() – クライアント上の全リアルタイムチャネルを一括で購読解除し、削除します ￼。すべてのチャネルがクローズされ、各チャネルの終了ステータスを含む配列をPromiseで返します。
	•	RealtimeChannel クラス – RealtimeClient.channel()で作成されるチャネルオブジェクトで、特定のトピックに対するリアルタイム通信を管理します。主要メソッド:
	•	subscribe(callback?: (status: 'SUBSCRIBED'|'TIMED_OUT'|'CLOSED'|'CHANNEL_ERROR', error?: Error) => void) – チャネルへの接続（サブスクライブ）を開始します。オプションでcallbackを渡すと、接続状態が変化する度に呼ばれます。例えばstatus === 'SUBSCRIBED'で購読成功、'CHANNEL_ERROR'でエラー発生、'TIMED_OUT'で接続タイムアウトした場合にそれぞれ処理できます ￼。callbackを省略した場合、メソッド呼び出し自体が接続完了を待つPromiseを返します。
	•	on(eventType: 'broadcast'|'presence'|'postgres_changes', filter: object, callback: Function) – チャネル上の特定のイベントを購読します。eventTypeに'broadcast'（ブロードキャストメッセージ）, 'presence'（プレゼンス参加状況）, 'postgres_changes'（データベース変更通知）を指定し、それぞれに応じたfilterオブジェクトを渡します。例えば:
	•	'broadcast'の場合、{ event: '<イベント名>' }をfilterに指定し、対応するイベント名のメッセージ受信時にcallback(payload)が実行されます ￼。
	•	'presence'の場合、{ event: 'sync'|'join'|'leave' }を指定し、他ユーザーの現在オンライン一覧取得（sync）、参加（join）、離脱（leave）の各イベント時にcallbackが呼ばれます ￼。
	•	'postgres_changes'の場合、{ event: 'INSERT'|'UPDATE'|'DELETE'|'*', schema: '<スキーマ名>', table?: '<テーブル名>', filter?: '<条件>' }を指定し、該当するデータ変更が発生したときにcallback(payload)が実行されます ￼。たとえば全テーブルの変更を監視するにはevent:'*', schema:'public'、特定テーブルのINSERTのみならevent:'INSERT', schema:'public', table:'messages'のように指定します。
	•	このonメソッドはRealtimeChannel自身を返すため、メソッドチェーンで続けてsubscribe()を呼ぶこともできます。
	•	send(payload: { type: 'broadcast'; event: string; payload: any } | { type: 'presence'; event: 'update'; payload: any }) – BroadcastメッセージまたはPresence状態をチャンネルに送信します。主にtype:'broadcast'を指定して任意のデータを同じチャンネルに接続中の他クライアントに配信するために使います。例えばchannel.send({ type: 'broadcast', event: 'some-event', payload: {...} })のように呼ぶと、同じチャンネルでon('broadcast', { event: 'some-event' }, ...)を登録した全クライアントにメッセージが配信されます ￼。ack:trueオプションを有効にしていれば、Promiseはサーバーからの受領確認後にresolveします。
	•	track(state: object) – Presence機能で現在のユーザーの状態を共有（追跡）します。例えばchannel.track({ user_id: 1, status: 'online' })のように使用し、チャンネル内に参加している他のクライアントにもこの情報が通知されます ￼。Promiseとして実行結果（成功時は'ok'）が返されます。trackで共有した状態はpresenceState()で確認できます。
	•	untrack() – Presenceで共有している自分の状態を削除します。例えばユーザーがチャンネルを離脱する前にchannel.untrack()を呼ぶと、他のクライアントにはleaveイベントで通知されます。戻り値はPromiseで、track同様にステータス文字列を返します。
	•	presenceState() – 現在そのチャネルで認識されている全ユーザーのPresence状態を取得します。戻り値はユーザーごとの状態オブジェクトのリスト（またはマップ）です。たとえばchannel.presenceState()で、現在「オンライン」としてtrackされているユーザー一覧を取得できます ￼。

6. @supabase/storage-js
	•	StorageClient クラス – Supabase Storageサービスを操作するためのクライアントです。コンストラクタにはSupabaseストレージエンドポイントのURLと認証トークン等を指定します（通常supabase.storageとして利用します）。プロジェクト内のストレージバケットの作成・管理、およびファイル入出力用のメソッドを提供します:
	•	createBucket(name: string, options?: { public: boolean }) – 新しいストレージバケットを作成します。nameはバケット名（ユニークな必要があります）、options.publicをtrueにすると公開バケット（誰でも読み取り可能）として作成します。成功時はバケットの情報をdataとして含むレスポンスを返します ￼。
	•	getBucket(name: string) – 指定したバケットの詳細情報を取得します。存在しない場合はerrorとしてエラーが返ります ￼。
	•	listBuckets() – プロジェクト内のすべてのバケット一覧を取得します。戻り値のdataに各バケットの情報配列が含まれます ￼。
	•	updateBucket(name: string, options: { public: boolean }) – バケットの公開設定を更新します。publicをtrue/falseに設定して既存バケットの公開/非公開を切り替えられます ￼。成功すれば更新後のバケット情報を返します。
	•	emptyBucket(name: string) – 指定したバケット内の全オブジェクトを削除します。一括削除が行われ、成功すればdataは空オブジェクト、errorがnullとなります ￼。
	•	deleteBucket(name: string) – 指定したバケットを削除します。バケット内にオブジェクトが残っている場合削除できないため、事前にemptyBucketする必要があります ￼。成功時dataに削除されたバケットの情報を返します。
	•	StorageBucketAPI（バケットファイル操作オブジェクト） – StorageClient.from(bucketName)で取得する、そのバケット内のファイルを操作するためのオブジェクトです。実際にはStorageFileApiクラスのインスタンスであり、以下のメソッドを提供します:
	•	upload(path: string, fileBody: File|Blob|Buffer, options?: { cacheControl?: string; upsert?: boolean }) – バケットにファイルをアップロードします。pathはバケット内での保存先パス（ファイル名含む）を指定します。fileBodyにはブラウザではFileやBlob、サーバではバッファなどを指定します。upsert:trueを指定すると同名ファイルが存在する場合上書きします。実行結果としてアップロードされたファイルの情報（KeyやbucketIdなど）がdataに含まれます ￼。
	•	download(path: string) – バケット内の指定パスのファイルをダウンロードします。戻り値dataにファイルのBlobオブジェクト（Node環境ではBuffer）が格納されます ￼。
	•	list(path?: string, options?: { limit?: number; offset?: number; sortBy?: { column: 'name'|'created_at'|'updated_at'; order: 'asc'|'desc' } }) – バケット内のファイル一覧を取得します。pathを指定するとそのフォルダ（プレフィックス）配下の項目に限定できます。limitとoffsetでページネーション、sortByで結果の並び順を指定可能です。戻り値dataにはファイル情報（名前、サイズ、更新日時など）の配列が入ります ￼。
	•	update(path: string, fileBody: File|Blob|Buffer, options?: { cacheControl?: string }) – バケット内の指定ファイルを新しい内容で更新（差し替え）します。基本的にuploadのupsert:trueと同等で、指定パスにファイルがなければ新規作成、あれば内容を置き換えます ￼。成功時には新しいファイルの情報を返します。
	•	move(fromPath: string, toPath: string) – バケット内でファイルを移動（リネーム）します。fromPathのファイルをtoPathのパスへ移動します。実行後は元のパスのファイルは削除され、新しいパスにファイルが存在します ￼。
	•	copy(fromPath: string, toPath: string) – バケット内でファイルをコピーします。fromPathのファイルをtoPathに複製します。成功すれば新しいコピー先ファイルの情報を返します（元のファイルは残ります）。
	•	remove(paths: string[]) – 複数のファイルを削除します。一つ以上のファイルパスの配列を渡すと、該当する全ファイルを削除します ￼。戻り値dataには削除に成功したファイル名一覧が含まれます。
	•	createSignedUrl(path: string, expiresIn: number) – 私有バケットのファイルに一時的にアクセスするための署名付きURLを発行します。expiresInはURLの有効期間（秒）で、例えば60を指定すると1分間有効なHTTPアクセスURLを取得できます ￼。戻り値data.signedURLにそのURL文字列が含まれます。
	•	createSignedUrls(paths: string[], expiresIn: number) – 複数ファイルに対して署名付きURLを一括発行します。pathsにファイルパスの配列を指定し、各々の有効期限はexpiresIn秒となります。戻り値dataはファイルごとの署名URL情報の配列（signedURLやエラー情報を含む）です。
	•	getPublicUrl(path: string) – 公開バケット内のファイルについて、パスに対応する公開URLを取得します。戻り値data.publicURLにそのファイルを直接参照できるURLが入ります ￼。このURLはバケットがパブリックの場合のみ有効です（私有バケットの場合はcreateSignedUrlを利用）。

以上がSupabase各パッケージv2系の主要なクラス・関数とメソッドの概要です。それぞれのメソッドはPromiseベースで動作し、成功時にはdata、失敗時にはerrorオブジェクトを含む結果を返す点が共通しています。 ￼ ￼