// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'models.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

T _$identity<T>(T value) => value;

final _privateConstructorUsedError = UnsupportedError(
  'It seems like you constructed your class using `MyClass._()`. This constructor is only meant to be used by freezed and you are not supposed to need it nor use it.\nPlease check the documentation here for more information: https://github.com/rrousselGit/freezed#adding-getters-and-methods-to-our-models',
);

/// @nodoc
mixin _$InstanceEvent {
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(TaskInstance field0) started,
    required TResult Function(String instanceId, String text) output,
    required TResult Function(TaskInstance field0) exited,
    required TResult Function(String instanceId, String message) error,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(TaskInstance field0)? started,
    TResult? Function(String instanceId, String text)? output,
    TResult? Function(TaskInstance field0)? exited,
    TResult? Function(String instanceId, String message)? error,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(TaskInstance field0)? started,
    TResult Function(String instanceId, String text)? output,
    TResult Function(TaskInstance field0)? exited,
    TResult Function(String instanceId, String message)? error,
    required TResult orElse(),
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(InstanceEvent_Started value) started,
    required TResult Function(InstanceEvent_Output value) output,
    required TResult Function(InstanceEvent_Exited value) exited,
    required TResult Function(InstanceEvent_Error value) error,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(InstanceEvent_Started value)? started,
    TResult? Function(InstanceEvent_Output value)? output,
    TResult? Function(InstanceEvent_Exited value)? exited,
    TResult? Function(InstanceEvent_Error value)? error,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(InstanceEvent_Started value)? started,
    TResult Function(InstanceEvent_Output value)? output,
    TResult Function(InstanceEvent_Exited value)? exited,
    TResult Function(InstanceEvent_Error value)? error,
    required TResult orElse(),
  }) => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $InstanceEventCopyWith<$Res> {
  factory $InstanceEventCopyWith(
    InstanceEvent value,
    $Res Function(InstanceEvent) then,
  ) = _$InstanceEventCopyWithImpl<$Res, InstanceEvent>;
}

/// @nodoc
class _$InstanceEventCopyWithImpl<$Res, $Val extends InstanceEvent>
    implements $InstanceEventCopyWith<$Res> {
  _$InstanceEventCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of InstanceEvent
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$InstanceEvent_StartedImplCopyWith<$Res> {
  factory _$$InstanceEvent_StartedImplCopyWith(
    _$InstanceEvent_StartedImpl value,
    $Res Function(_$InstanceEvent_StartedImpl) then,
  ) = __$$InstanceEvent_StartedImplCopyWithImpl<$Res>;
  @useResult
  $Res call({TaskInstance field0});
}

/// @nodoc
class __$$InstanceEvent_StartedImplCopyWithImpl<$Res>
    extends _$InstanceEventCopyWithImpl<$Res, _$InstanceEvent_StartedImpl>
    implements _$$InstanceEvent_StartedImplCopyWith<$Res> {
  __$$InstanceEvent_StartedImplCopyWithImpl(
    _$InstanceEvent_StartedImpl _value,
    $Res Function(_$InstanceEvent_StartedImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of InstanceEvent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? field0 = null}) {
    return _then(
      _$InstanceEvent_StartedImpl(
        null == field0
            ? _value.field0
            : field0 // ignore: cast_nullable_to_non_nullable
                  as TaskInstance,
      ),
    );
  }
}

/// @nodoc

class _$InstanceEvent_StartedImpl extends InstanceEvent_Started {
  const _$InstanceEvent_StartedImpl(this.field0) : super._();

  @override
  final TaskInstance field0;

  @override
  String toString() {
    return 'InstanceEvent.started(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$InstanceEvent_StartedImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of InstanceEvent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$InstanceEvent_StartedImplCopyWith<_$InstanceEvent_StartedImpl>
  get copyWith =>
      __$$InstanceEvent_StartedImplCopyWithImpl<_$InstanceEvent_StartedImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(TaskInstance field0) started,
    required TResult Function(String instanceId, String text) output,
    required TResult Function(TaskInstance field0) exited,
    required TResult Function(String instanceId, String message) error,
  }) {
    return started(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(TaskInstance field0)? started,
    TResult? Function(String instanceId, String text)? output,
    TResult? Function(TaskInstance field0)? exited,
    TResult? Function(String instanceId, String message)? error,
  }) {
    return started?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(TaskInstance field0)? started,
    TResult Function(String instanceId, String text)? output,
    TResult Function(TaskInstance field0)? exited,
    TResult Function(String instanceId, String message)? error,
    required TResult orElse(),
  }) {
    if (started != null) {
      return started(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(InstanceEvent_Started value) started,
    required TResult Function(InstanceEvent_Output value) output,
    required TResult Function(InstanceEvent_Exited value) exited,
    required TResult Function(InstanceEvent_Error value) error,
  }) {
    return started(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(InstanceEvent_Started value)? started,
    TResult? Function(InstanceEvent_Output value)? output,
    TResult? Function(InstanceEvent_Exited value)? exited,
    TResult? Function(InstanceEvent_Error value)? error,
  }) {
    return started?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(InstanceEvent_Started value)? started,
    TResult Function(InstanceEvent_Output value)? output,
    TResult Function(InstanceEvent_Exited value)? exited,
    TResult Function(InstanceEvent_Error value)? error,
    required TResult orElse(),
  }) {
    if (started != null) {
      return started(this);
    }
    return orElse();
  }
}

abstract class InstanceEvent_Started extends InstanceEvent {
  const factory InstanceEvent_Started(final TaskInstance field0) =
      _$InstanceEvent_StartedImpl;
  const InstanceEvent_Started._() : super._();

  TaskInstance get field0;

  /// Create a copy of InstanceEvent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$InstanceEvent_StartedImplCopyWith<_$InstanceEvent_StartedImpl>
  get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$InstanceEvent_OutputImplCopyWith<$Res> {
  factory _$$InstanceEvent_OutputImplCopyWith(
    _$InstanceEvent_OutputImpl value,
    $Res Function(_$InstanceEvent_OutputImpl) then,
  ) = __$$InstanceEvent_OutputImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String instanceId, String text});
}

/// @nodoc
class __$$InstanceEvent_OutputImplCopyWithImpl<$Res>
    extends _$InstanceEventCopyWithImpl<$Res, _$InstanceEvent_OutputImpl>
    implements _$$InstanceEvent_OutputImplCopyWith<$Res> {
  __$$InstanceEvent_OutputImplCopyWithImpl(
    _$InstanceEvent_OutputImpl _value,
    $Res Function(_$InstanceEvent_OutputImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of InstanceEvent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? instanceId = null, Object? text = null}) {
    return _then(
      _$InstanceEvent_OutputImpl(
        instanceId: null == instanceId
            ? _value.instanceId
            : instanceId // ignore: cast_nullable_to_non_nullable
                  as String,
        text: null == text
            ? _value.text
            : text // ignore: cast_nullable_to_non_nullable
                  as String,
      ),
    );
  }
}

/// @nodoc

class _$InstanceEvent_OutputImpl extends InstanceEvent_Output {
  const _$InstanceEvent_OutputImpl({
    required this.instanceId,
    required this.text,
  }) : super._();

  @override
  final String instanceId;

  /// UTF-8 解码后的文本增量
  @override
  final String text;

  @override
  String toString() {
    return 'InstanceEvent.output(instanceId: $instanceId, text: $text)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$InstanceEvent_OutputImpl &&
            (identical(other.instanceId, instanceId) ||
                other.instanceId == instanceId) &&
            (identical(other.text, text) || other.text == text));
  }

  @override
  int get hashCode => Object.hash(runtimeType, instanceId, text);

  /// Create a copy of InstanceEvent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$InstanceEvent_OutputImplCopyWith<_$InstanceEvent_OutputImpl>
  get copyWith =>
      __$$InstanceEvent_OutputImplCopyWithImpl<_$InstanceEvent_OutputImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(TaskInstance field0) started,
    required TResult Function(String instanceId, String text) output,
    required TResult Function(TaskInstance field0) exited,
    required TResult Function(String instanceId, String message) error,
  }) {
    return output(instanceId, text);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(TaskInstance field0)? started,
    TResult? Function(String instanceId, String text)? output,
    TResult? Function(TaskInstance field0)? exited,
    TResult? Function(String instanceId, String message)? error,
  }) {
    return output?.call(instanceId, text);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(TaskInstance field0)? started,
    TResult Function(String instanceId, String text)? output,
    TResult Function(TaskInstance field0)? exited,
    TResult Function(String instanceId, String message)? error,
    required TResult orElse(),
  }) {
    if (output != null) {
      return output(instanceId, text);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(InstanceEvent_Started value) started,
    required TResult Function(InstanceEvent_Output value) output,
    required TResult Function(InstanceEvent_Exited value) exited,
    required TResult Function(InstanceEvent_Error value) error,
  }) {
    return output(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(InstanceEvent_Started value)? started,
    TResult? Function(InstanceEvent_Output value)? output,
    TResult? Function(InstanceEvent_Exited value)? exited,
    TResult? Function(InstanceEvent_Error value)? error,
  }) {
    return output?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(InstanceEvent_Started value)? started,
    TResult Function(InstanceEvent_Output value)? output,
    TResult Function(InstanceEvent_Exited value)? exited,
    TResult Function(InstanceEvent_Error value)? error,
    required TResult orElse(),
  }) {
    if (output != null) {
      return output(this);
    }
    return orElse();
  }
}

abstract class InstanceEvent_Output extends InstanceEvent {
  const factory InstanceEvent_Output({
    required final String instanceId,
    required final String text,
  }) = _$InstanceEvent_OutputImpl;
  const InstanceEvent_Output._() : super._();

  String get instanceId;

  /// UTF-8 解码后的文本增量
  String get text;

  /// Create a copy of InstanceEvent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$InstanceEvent_OutputImplCopyWith<_$InstanceEvent_OutputImpl>
  get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$InstanceEvent_ExitedImplCopyWith<$Res> {
  factory _$$InstanceEvent_ExitedImplCopyWith(
    _$InstanceEvent_ExitedImpl value,
    $Res Function(_$InstanceEvent_ExitedImpl) then,
  ) = __$$InstanceEvent_ExitedImplCopyWithImpl<$Res>;
  @useResult
  $Res call({TaskInstance field0});
}

/// @nodoc
class __$$InstanceEvent_ExitedImplCopyWithImpl<$Res>
    extends _$InstanceEventCopyWithImpl<$Res, _$InstanceEvent_ExitedImpl>
    implements _$$InstanceEvent_ExitedImplCopyWith<$Res> {
  __$$InstanceEvent_ExitedImplCopyWithImpl(
    _$InstanceEvent_ExitedImpl _value,
    $Res Function(_$InstanceEvent_ExitedImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of InstanceEvent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? field0 = null}) {
    return _then(
      _$InstanceEvent_ExitedImpl(
        null == field0
            ? _value.field0
            : field0 // ignore: cast_nullable_to_non_nullable
                  as TaskInstance,
      ),
    );
  }
}

/// @nodoc

class _$InstanceEvent_ExitedImpl extends InstanceEvent_Exited {
  const _$InstanceEvent_ExitedImpl(this.field0) : super._();

  @override
  final TaskInstance field0;

  @override
  String toString() {
    return 'InstanceEvent.exited(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$InstanceEvent_ExitedImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of InstanceEvent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$InstanceEvent_ExitedImplCopyWith<_$InstanceEvent_ExitedImpl>
  get copyWith =>
      __$$InstanceEvent_ExitedImplCopyWithImpl<_$InstanceEvent_ExitedImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(TaskInstance field0) started,
    required TResult Function(String instanceId, String text) output,
    required TResult Function(TaskInstance field0) exited,
    required TResult Function(String instanceId, String message) error,
  }) {
    return exited(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(TaskInstance field0)? started,
    TResult? Function(String instanceId, String text)? output,
    TResult? Function(TaskInstance field0)? exited,
    TResult? Function(String instanceId, String message)? error,
  }) {
    return exited?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(TaskInstance field0)? started,
    TResult Function(String instanceId, String text)? output,
    TResult Function(TaskInstance field0)? exited,
    TResult Function(String instanceId, String message)? error,
    required TResult orElse(),
  }) {
    if (exited != null) {
      return exited(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(InstanceEvent_Started value) started,
    required TResult Function(InstanceEvent_Output value) output,
    required TResult Function(InstanceEvent_Exited value) exited,
    required TResult Function(InstanceEvent_Error value) error,
  }) {
    return exited(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(InstanceEvent_Started value)? started,
    TResult? Function(InstanceEvent_Output value)? output,
    TResult? Function(InstanceEvent_Exited value)? exited,
    TResult? Function(InstanceEvent_Error value)? error,
  }) {
    return exited?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(InstanceEvent_Started value)? started,
    TResult Function(InstanceEvent_Output value)? output,
    TResult Function(InstanceEvent_Exited value)? exited,
    TResult Function(InstanceEvent_Error value)? error,
    required TResult orElse(),
  }) {
    if (exited != null) {
      return exited(this);
    }
    return orElse();
  }
}

abstract class InstanceEvent_Exited extends InstanceEvent {
  const factory InstanceEvent_Exited(final TaskInstance field0) =
      _$InstanceEvent_ExitedImpl;
  const InstanceEvent_Exited._() : super._();

  TaskInstance get field0;

  /// Create a copy of InstanceEvent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$InstanceEvent_ExitedImplCopyWith<_$InstanceEvent_ExitedImpl>
  get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$InstanceEvent_ErrorImplCopyWith<$Res> {
  factory _$$InstanceEvent_ErrorImplCopyWith(
    _$InstanceEvent_ErrorImpl value,
    $Res Function(_$InstanceEvent_ErrorImpl) then,
  ) = __$$InstanceEvent_ErrorImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String instanceId, String message});
}

/// @nodoc
class __$$InstanceEvent_ErrorImplCopyWithImpl<$Res>
    extends _$InstanceEventCopyWithImpl<$Res, _$InstanceEvent_ErrorImpl>
    implements _$$InstanceEvent_ErrorImplCopyWith<$Res> {
  __$$InstanceEvent_ErrorImplCopyWithImpl(
    _$InstanceEvent_ErrorImpl _value,
    $Res Function(_$InstanceEvent_ErrorImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of InstanceEvent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? instanceId = null, Object? message = null}) {
    return _then(
      _$InstanceEvent_ErrorImpl(
        instanceId: null == instanceId
            ? _value.instanceId
            : instanceId // ignore: cast_nullable_to_non_nullable
                  as String,
        message: null == message
            ? _value.message
            : message // ignore: cast_nullable_to_non_nullable
                  as String,
      ),
    );
  }
}

/// @nodoc

class _$InstanceEvent_ErrorImpl extends InstanceEvent_Error {
  const _$InstanceEvent_ErrorImpl({
    required this.instanceId,
    required this.message,
  }) : super._();

  @override
  final String instanceId;
  @override
  final String message;

  @override
  String toString() {
    return 'InstanceEvent.error(instanceId: $instanceId, message: $message)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$InstanceEvent_ErrorImpl &&
            (identical(other.instanceId, instanceId) ||
                other.instanceId == instanceId) &&
            (identical(other.message, message) || other.message == message));
  }

  @override
  int get hashCode => Object.hash(runtimeType, instanceId, message);

  /// Create a copy of InstanceEvent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$InstanceEvent_ErrorImplCopyWith<_$InstanceEvent_ErrorImpl> get copyWith =>
      __$$InstanceEvent_ErrorImplCopyWithImpl<_$InstanceEvent_ErrorImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(TaskInstance field0) started,
    required TResult Function(String instanceId, String text) output,
    required TResult Function(TaskInstance field0) exited,
    required TResult Function(String instanceId, String message) error,
  }) {
    return error(instanceId, message);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(TaskInstance field0)? started,
    TResult? Function(String instanceId, String text)? output,
    TResult? Function(TaskInstance field0)? exited,
    TResult? Function(String instanceId, String message)? error,
  }) {
    return error?.call(instanceId, message);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(TaskInstance field0)? started,
    TResult Function(String instanceId, String text)? output,
    TResult Function(TaskInstance field0)? exited,
    TResult Function(String instanceId, String message)? error,
    required TResult orElse(),
  }) {
    if (error != null) {
      return error(instanceId, message);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(InstanceEvent_Started value) started,
    required TResult Function(InstanceEvent_Output value) output,
    required TResult Function(InstanceEvent_Exited value) exited,
    required TResult Function(InstanceEvent_Error value) error,
  }) {
    return error(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(InstanceEvent_Started value)? started,
    TResult? Function(InstanceEvent_Output value)? output,
    TResult? Function(InstanceEvent_Exited value)? exited,
    TResult? Function(InstanceEvent_Error value)? error,
  }) {
    return error?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(InstanceEvent_Started value)? started,
    TResult Function(InstanceEvent_Output value)? output,
    TResult Function(InstanceEvent_Exited value)? exited,
    TResult Function(InstanceEvent_Error value)? error,
    required TResult orElse(),
  }) {
    if (error != null) {
      return error(this);
    }
    return orElse();
  }
}

abstract class InstanceEvent_Error extends InstanceEvent {
  const factory InstanceEvent_Error({
    required final String instanceId,
    required final String message,
  }) = _$InstanceEvent_ErrorImpl;
  const InstanceEvent_Error._() : super._();

  String get instanceId;
  String get message;

  /// Create a copy of InstanceEvent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$InstanceEvent_ErrorImplCopyWith<_$InstanceEvent_ErrorImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$InstanceStatus {
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() running,
    required TResult Function(int code) exited,
    required TResult Function() killed,
    required TResult Function(String message) error,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? running,
    TResult? Function(int code)? exited,
    TResult? Function()? killed,
    TResult? Function(String message)? error,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? running,
    TResult Function(int code)? exited,
    TResult Function()? killed,
    TResult Function(String message)? error,
    required TResult orElse(),
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(InstanceStatus_Running value) running,
    required TResult Function(InstanceStatus_Exited value) exited,
    required TResult Function(InstanceStatus_Killed value) killed,
    required TResult Function(InstanceStatus_Error value) error,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(InstanceStatus_Running value)? running,
    TResult? Function(InstanceStatus_Exited value)? exited,
    TResult? Function(InstanceStatus_Killed value)? killed,
    TResult? Function(InstanceStatus_Error value)? error,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(InstanceStatus_Running value)? running,
    TResult Function(InstanceStatus_Exited value)? exited,
    TResult Function(InstanceStatus_Killed value)? killed,
    TResult Function(InstanceStatus_Error value)? error,
    required TResult orElse(),
  }) => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $InstanceStatusCopyWith<$Res> {
  factory $InstanceStatusCopyWith(
    InstanceStatus value,
    $Res Function(InstanceStatus) then,
  ) = _$InstanceStatusCopyWithImpl<$Res, InstanceStatus>;
}

/// @nodoc
class _$InstanceStatusCopyWithImpl<$Res, $Val extends InstanceStatus>
    implements $InstanceStatusCopyWith<$Res> {
  _$InstanceStatusCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of InstanceStatus
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$InstanceStatus_RunningImplCopyWith<$Res> {
  factory _$$InstanceStatus_RunningImplCopyWith(
    _$InstanceStatus_RunningImpl value,
    $Res Function(_$InstanceStatus_RunningImpl) then,
  ) = __$$InstanceStatus_RunningImplCopyWithImpl<$Res>;
}

/// @nodoc
class __$$InstanceStatus_RunningImplCopyWithImpl<$Res>
    extends _$InstanceStatusCopyWithImpl<$Res, _$InstanceStatus_RunningImpl>
    implements _$$InstanceStatus_RunningImplCopyWith<$Res> {
  __$$InstanceStatus_RunningImplCopyWithImpl(
    _$InstanceStatus_RunningImpl _value,
    $Res Function(_$InstanceStatus_RunningImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of InstanceStatus
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc

class _$InstanceStatus_RunningImpl extends InstanceStatus_Running {
  const _$InstanceStatus_RunningImpl() : super._();

  @override
  String toString() {
    return 'InstanceStatus.running()';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$InstanceStatus_RunningImpl);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() running,
    required TResult Function(int code) exited,
    required TResult Function() killed,
    required TResult Function(String message) error,
  }) {
    return running();
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? running,
    TResult? Function(int code)? exited,
    TResult? Function()? killed,
    TResult? Function(String message)? error,
  }) {
    return running?.call();
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? running,
    TResult Function(int code)? exited,
    TResult Function()? killed,
    TResult Function(String message)? error,
    required TResult orElse(),
  }) {
    if (running != null) {
      return running();
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(InstanceStatus_Running value) running,
    required TResult Function(InstanceStatus_Exited value) exited,
    required TResult Function(InstanceStatus_Killed value) killed,
    required TResult Function(InstanceStatus_Error value) error,
  }) {
    return running(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(InstanceStatus_Running value)? running,
    TResult? Function(InstanceStatus_Exited value)? exited,
    TResult? Function(InstanceStatus_Killed value)? killed,
    TResult? Function(InstanceStatus_Error value)? error,
  }) {
    return running?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(InstanceStatus_Running value)? running,
    TResult Function(InstanceStatus_Exited value)? exited,
    TResult Function(InstanceStatus_Killed value)? killed,
    TResult Function(InstanceStatus_Error value)? error,
    required TResult orElse(),
  }) {
    if (running != null) {
      return running(this);
    }
    return orElse();
  }
}

abstract class InstanceStatus_Running extends InstanceStatus {
  const factory InstanceStatus_Running() = _$InstanceStatus_RunningImpl;
  const InstanceStatus_Running._() : super._();
}

/// @nodoc
abstract class _$$InstanceStatus_ExitedImplCopyWith<$Res> {
  factory _$$InstanceStatus_ExitedImplCopyWith(
    _$InstanceStatus_ExitedImpl value,
    $Res Function(_$InstanceStatus_ExitedImpl) then,
  ) = __$$InstanceStatus_ExitedImplCopyWithImpl<$Res>;
  @useResult
  $Res call({int code});
}

/// @nodoc
class __$$InstanceStatus_ExitedImplCopyWithImpl<$Res>
    extends _$InstanceStatusCopyWithImpl<$Res, _$InstanceStatus_ExitedImpl>
    implements _$$InstanceStatus_ExitedImplCopyWith<$Res> {
  __$$InstanceStatus_ExitedImplCopyWithImpl(
    _$InstanceStatus_ExitedImpl _value,
    $Res Function(_$InstanceStatus_ExitedImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of InstanceStatus
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? code = null}) {
    return _then(
      _$InstanceStatus_ExitedImpl(
        code: null == code
            ? _value.code
            : code // ignore: cast_nullable_to_non_nullable
                  as int,
      ),
    );
  }
}

/// @nodoc

class _$InstanceStatus_ExitedImpl extends InstanceStatus_Exited {
  const _$InstanceStatus_ExitedImpl({required this.code}) : super._();

  @override
  final int code;

  @override
  String toString() {
    return 'InstanceStatus.exited(code: $code)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$InstanceStatus_ExitedImpl &&
            (identical(other.code, code) || other.code == code));
  }

  @override
  int get hashCode => Object.hash(runtimeType, code);

  /// Create a copy of InstanceStatus
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$InstanceStatus_ExitedImplCopyWith<_$InstanceStatus_ExitedImpl>
  get copyWith =>
      __$$InstanceStatus_ExitedImplCopyWithImpl<_$InstanceStatus_ExitedImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() running,
    required TResult Function(int code) exited,
    required TResult Function() killed,
    required TResult Function(String message) error,
  }) {
    return exited(code);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? running,
    TResult? Function(int code)? exited,
    TResult? Function()? killed,
    TResult? Function(String message)? error,
  }) {
    return exited?.call(code);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? running,
    TResult Function(int code)? exited,
    TResult Function()? killed,
    TResult Function(String message)? error,
    required TResult orElse(),
  }) {
    if (exited != null) {
      return exited(code);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(InstanceStatus_Running value) running,
    required TResult Function(InstanceStatus_Exited value) exited,
    required TResult Function(InstanceStatus_Killed value) killed,
    required TResult Function(InstanceStatus_Error value) error,
  }) {
    return exited(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(InstanceStatus_Running value)? running,
    TResult? Function(InstanceStatus_Exited value)? exited,
    TResult? Function(InstanceStatus_Killed value)? killed,
    TResult? Function(InstanceStatus_Error value)? error,
  }) {
    return exited?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(InstanceStatus_Running value)? running,
    TResult Function(InstanceStatus_Exited value)? exited,
    TResult Function(InstanceStatus_Killed value)? killed,
    TResult Function(InstanceStatus_Error value)? error,
    required TResult orElse(),
  }) {
    if (exited != null) {
      return exited(this);
    }
    return orElse();
  }
}

abstract class InstanceStatus_Exited extends InstanceStatus {
  const factory InstanceStatus_Exited({required final int code}) =
      _$InstanceStatus_ExitedImpl;
  const InstanceStatus_Exited._() : super._();

  int get code;

  /// Create a copy of InstanceStatus
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$InstanceStatus_ExitedImplCopyWith<_$InstanceStatus_ExitedImpl>
  get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$InstanceStatus_KilledImplCopyWith<$Res> {
  factory _$$InstanceStatus_KilledImplCopyWith(
    _$InstanceStatus_KilledImpl value,
    $Res Function(_$InstanceStatus_KilledImpl) then,
  ) = __$$InstanceStatus_KilledImplCopyWithImpl<$Res>;
}

/// @nodoc
class __$$InstanceStatus_KilledImplCopyWithImpl<$Res>
    extends _$InstanceStatusCopyWithImpl<$Res, _$InstanceStatus_KilledImpl>
    implements _$$InstanceStatus_KilledImplCopyWith<$Res> {
  __$$InstanceStatus_KilledImplCopyWithImpl(
    _$InstanceStatus_KilledImpl _value,
    $Res Function(_$InstanceStatus_KilledImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of InstanceStatus
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc

class _$InstanceStatus_KilledImpl extends InstanceStatus_Killed {
  const _$InstanceStatus_KilledImpl() : super._();

  @override
  String toString() {
    return 'InstanceStatus.killed()';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$InstanceStatus_KilledImpl);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() running,
    required TResult Function(int code) exited,
    required TResult Function() killed,
    required TResult Function(String message) error,
  }) {
    return killed();
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? running,
    TResult? Function(int code)? exited,
    TResult? Function()? killed,
    TResult? Function(String message)? error,
  }) {
    return killed?.call();
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? running,
    TResult Function(int code)? exited,
    TResult Function()? killed,
    TResult Function(String message)? error,
    required TResult orElse(),
  }) {
    if (killed != null) {
      return killed();
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(InstanceStatus_Running value) running,
    required TResult Function(InstanceStatus_Exited value) exited,
    required TResult Function(InstanceStatus_Killed value) killed,
    required TResult Function(InstanceStatus_Error value) error,
  }) {
    return killed(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(InstanceStatus_Running value)? running,
    TResult? Function(InstanceStatus_Exited value)? exited,
    TResult? Function(InstanceStatus_Killed value)? killed,
    TResult? Function(InstanceStatus_Error value)? error,
  }) {
    return killed?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(InstanceStatus_Running value)? running,
    TResult Function(InstanceStatus_Exited value)? exited,
    TResult Function(InstanceStatus_Killed value)? killed,
    TResult Function(InstanceStatus_Error value)? error,
    required TResult orElse(),
  }) {
    if (killed != null) {
      return killed(this);
    }
    return orElse();
  }
}

abstract class InstanceStatus_Killed extends InstanceStatus {
  const factory InstanceStatus_Killed() = _$InstanceStatus_KilledImpl;
  const InstanceStatus_Killed._() : super._();
}

/// @nodoc
abstract class _$$InstanceStatus_ErrorImplCopyWith<$Res> {
  factory _$$InstanceStatus_ErrorImplCopyWith(
    _$InstanceStatus_ErrorImpl value,
    $Res Function(_$InstanceStatus_ErrorImpl) then,
  ) = __$$InstanceStatus_ErrorImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String message});
}

/// @nodoc
class __$$InstanceStatus_ErrorImplCopyWithImpl<$Res>
    extends _$InstanceStatusCopyWithImpl<$Res, _$InstanceStatus_ErrorImpl>
    implements _$$InstanceStatus_ErrorImplCopyWith<$Res> {
  __$$InstanceStatus_ErrorImplCopyWithImpl(
    _$InstanceStatus_ErrorImpl _value,
    $Res Function(_$InstanceStatus_ErrorImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of InstanceStatus
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? message = null}) {
    return _then(
      _$InstanceStatus_ErrorImpl(
        message: null == message
            ? _value.message
            : message // ignore: cast_nullable_to_non_nullable
                  as String,
      ),
    );
  }
}

/// @nodoc

class _$InstanceStatus_ErrorImpl extends InstanceStatus_Error {
  const _$InstanceStatus_ErrorImpl({required this.message}) : super._();

  @override
  final String message;

  @override
  String toString() {
    return 'InstanceStatus.error(message: $message)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$InstanceStatus_ErrorImpl &&
            (identical(other.message, message) || other.message == message));
  }

  @override
  int get hashCode => Object.hash(runtimeType, message);

  /// Create a copy of InstanceStatus
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$InstanceStatus_ErrorImplCopyWith<_$InstanceStatus_ErrorImpl>
  get copyWith =>
      __$$InstanceStatus_ErrorImplCopyWithImpl<_$InstanceStatus_ErrorImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() running,
    required TResult Function(int code) exited,
    required TResult Function() killed,
    required TResult Function(String message) error,
  }) {
    return error(message);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? running,
    TResult? Function(int code)? exited,
    TResult? Function()? killed,
    TResult? Function(String message)? error,
  }) {
    return error?.call(message);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? running,
    TResult Function(int code)? exited,
    TResult Function()? killed,
    TResult Function(String message)? error,
    required TResult orElse(),
  }) {
    if (error != null) {
      return error(message);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(InstanceStatus_Running value) running,
    required TResult Function(InstanceStatus_Exited value) exited,
    required TResult Function(InstanceStatus_Killed value) killed,
    required TResult Function(InstanceStatus_Error value) error,
  }) {
    return error(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(InstanceStatus_Running value)? running,
    TResult? Function(InstanceStatus_Exited value)? exited,
    TResult? Function(InstanceStatus_Killed value)? killed,
    TResult? Function(InstanceStatus_Error value)? error,
  }) {
    return error?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(InstanceStatus_Running value)? running,
    TResult Function(InstanceStatus_Exited value)? exited,
    TResult Function(InstanceStatus_Killed value)? killed,
    TResult Function(InstanceStatus_Error value)? error,
    required TResult orElse(),
  }) {
    if (error != null) {
      return error(this);
    }
    return orElse();
  }
}

abstract class InstanceStatus_Error extends InstanceStatus {
  const factory InstanceStatus_Error({required final String message}) =
      _$InstanceStatus_ErrorImpl;
  const InstanceStatus_Error._() : super._();

  String get message;

  /// Create a copy of InstanceStatus
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$InstanceStatus_ErrorImplCopyWith<_$InstanceStatus_ErrorImpl>
  get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$StepState {
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() pending,
    required TResult Function(String instanceId) running,
    required TResult Function(String instanceId, int exitCode) completed,
    required TResult Function(String error) failed,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? pending,
    TResult? Function(String instanceId)? running,
    TResult? Function(String instanceId, int exitCode)? completed,
    TResult? Function(String error)? failed,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? pending,
    TResult Function(String instanceId)? running,
    TResult Function(String instanceId, int exitCode)? completed,
    TResult Function(String error)? failed,
    required TResult orElse(),
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(StepState_Pending value) pending,
    required TResult Function(StepState_Running value) running,
    required TResult Function(StepState_Completed value) completed,
    required TResult Function(StepState_Failed value) failed,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(StepState_Pending value)? pending,
    TResult? Function(StepState_Running value)? running,
    TResult? Function(StepState_Completed value)? completed,
    TResult? Function(StepState_Failed value)? failed,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(StepState_Pending value)? pending,
    TResult Function(StepState_Running value)? running,
    TResult Function(StepState_Completed value)? completed,
    TResult Function(StepState_Failed value)? failed,
    required TResult orElse(),
  }) => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $StepStateCopyWith<$Res> {
  factory $StepStateCopyWith(StepState value, $Res Function(StepState) then) =
      _$StepStateCopyWithImpl<$Res, StepState>;
}

/// @nodoc
class _$StepStateCopyWithImpl<$Res, $Val extends StepState>
    implements $StepStateCopyWith<$Res> {
  _$StepStateCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of StepState
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$StepState_PendingImplCopyWith<$Res> {
  factory _$$StepState_PendingImplCopyWith(
    _$StepState_PendingImpl value,
    $Res Function(_$StepState_PendingImpl) then,
  ) = __$$StepState_PendingImplCopyWithImpl<$Res>;
}

/// @nodoc
class __$$StepState_PendingImplCopyWithImpl<$Res>
    extends _$StepStateCopyWithImpl<$Res, _$StepState_PendingImpl>
    implements _$$StepState_PendingImplCopyWith<$Res> {
  __$$StepState_PendingImplCopyWithImpl(
    _$StepState_PendingImpl _value,
    $Res Function(_$StepState_PendingImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of StepState
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc

class _$StepState_PendingImpl extends StepState_Pending {
  const _$StepState_PendingImpl() : super._();

  @override
  String toString() {
    return 'StepState.pending()';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is _$StepState_PendingImpl);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() pending,
    required TResult Function(String instanceId) running,
    required TResult Function(String instanceId, int exitCode) completed,
    required TResult Function(String error) failed,
  }) {
    return pending();
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? pending,
    TResult? Function(String instanceId)? running,
    TResult? Function(String instanceId, int exitCode)? completed,
    TResult? Function(String error)? failed,
  }) {
    return pending?.call();
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? pending,
    TResult Function(String instanceId)? running,
    TResult Function(String instanceId, int exitCode)? completed,
    TResult Function(String error)? failed,
    required TResult orElse(),
  }) {
    if (pending != null) {
      return pending();
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(StepState_Pending value) pending,
    required TResult Function(StepState_Running value) running,
    required TResult Function(StepState_Completed value) completed,
    required TResult Function(StepState_Failed value) failed,
  }) {
    return pending(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(StepState_Pending value)? pending,
    TResult? Function(StepState_Running value)? running,
    TResult? Function(StepState_Completed value)? completed,
    TResult? Function(StepState_Failed value)? failed,
  }) {
    return pending?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(StepState_Pending value)? pending,
    TResult Function(StepState_Running value)? running,
    TResult Function(StepState_Completed value)? completed,
    TResult Function(StepState_Failed value)? failed,
    required TResult orElse(),
  }) {
    if (pending != null) {
      return pending(this);
    }
    return orElse();
  }
}

abstract class StepState_Pending extends StepState {
  const factory StepState_Pending() = _$StepState_PendingImpl;
  const StepState_Pending._() : super._();
}

/// @nodoc
abstract class _$$StepState_RunningImplCopyWith<$Res> {
  factory _$$StepState_RunningImplCopyWith(
    _$StepState_RunningImpl value,
    $Res Function(_$StepState_RunningImpl) then,
  ) = __$$StepState_RunningImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String instanceId});
}

/// @nodoc
class __$$StepState_RunningImplCopyWithImpl<$Res>
    extends _$StepStateCopyWithImpl<$Res, _$StepState_RunningImpl>
    implements _$$StepState_RunningImplCopyWith<$Res> {
  __$$StepState_RunningImplCopyWithImpl(
    _$StepState_RunningImpl _value,
    $Res Function(_$StepState_RunningImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of StepState
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? instanceId = null}) {
    return _then(
      _$StepState_RunningImpl(
        instanceId: null == instanceId
            ? _value.instanceId
            : instanceId // ignore: cast_nullable_to_non_nullable
                  as String,
      ),
    );
  }
}

/// @nodoc

class _$StepState_RunningImpl extends StepState_Running {
  const _$StepState_RunningImpl({required this.instanceId}) : super._();

  @override
  final String instanceId;

  @override
  String toString() {
    return 'StepState.running(instanceId: $instanceId)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$StepState_RunningImpl &&
            (identical(other.instanceId, instanceId) ||
                other.instanceId == instanceId));
  }

  @override
  int get hashCode => Object.hash(runtimeType, instanceId);

  /// Create a copy of StepState
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$StepState_RunningImplCopyWith<_$StepState_RunningImpl> get copyWith =>
      __$$StepState_RunningImplCopyWithImpl<_$StepState_RunningImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() pending,
    required TResult Function(String instanceId) running,
    required TResult Function(String instanceId, int exitCode) completed,
    required TResult Function(String error) failed,
  }) {
    return running(instanceId);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? pending,
    TResult? Function(String instanceId)? running,
    TResult? Function(String instanceId, int exitCode)? completed,
    TResult? Function(String error)? failed,
  }) {
    return running?.call(instanceId);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? pending,
    TResult Function(String instanceId)? running,
    TResult Function(String instanceId, int exitCode)? completed,
    TResult Function(String error)? failed,
    required TResult orElse(),
  }) {
    if (running != null) {
      return running(instanceId);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(StepState_Pending value) pending,
    required TResult Function(StepState_Running value) running,
    required TResult Function(StepState_Completed value) completed,
    required TResult Function(StepState_Failed value) failed,
  }) {
    return running(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(StepState_Pending value)? pending,
    TResult? Function(StepState_Running value)? running,
    TResult? Function(StepState_Completed value)? completed,
    TResult? Function(StepState_Failed value)? failed,
  }) {
    return running?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(StepState_Pending value)? pending,
    TResult Function(StepState_Running value)? running,
    TResult Function(StepState_Completed value)? completed,
    TResult Function(StepState_Failed value)? failed,
    required TResult orElse(),
  }) {
    if (running != null) {
      return running(this);
    }
    return orElse();
  }
}

abstract class StepState_Running extends StepState {
  const factory StepState_Running({required final String instanceId}) =
      _$StepState_RunningImpl;
  const StepState_Running._() : super._();

  String get instanceId;

  /// Create a copy of StepState
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$StepState_RunningImplCopyWith<_$StepState_RunningImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$StepState_CompletedImplCopyWith<$Res> {
  factory _$$StepState_CompletedImplCopyWith(
    _$StepState_CompletedImpl value,
    $Res Function(_$StepState_CompletedImpl) then,
  ) = __$$StepState_CompletedImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String instanceId, int exitCode});
}

/// @nodoc
class __$$StepState_CompletedImplCopyWithImpl<$Res>
    extends _$StepStateCopyWithImpl<$Res, _$StepState_CompletedImpl>
    implements _$$StepState_CompletedImplCopyWith<$Res> {
  __$$StepState_CompletedImplCopyWithImpl(
    _$StepState_CompletedImpl _value,
    $Res Function(_$StepState_CompletedImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of StepState
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? instanceId = null, Object? exitCode = null}) {
    return _then(
      _$StepState_CompletedImpl(
        instanceId: null == instanceId
            ? _value.instanceId
            : instanceId // ignore: cast_nullable_to_non_nullable
                  as String,
        exitCode: null == exitCode
            ? _value.exitCode
            : exitCode // ignore: cast_nullable_to_non_nullable
                  as int,
      ),
    );
  }
}

/// @nodoc

class _$StepState_CompletedImpl extends StepState_Completed {
  const _$StepState_CompletedImpl({
    required this.instanceId,
    required this.exitCode,
  }) : super._();

  @override
  final String instanceId;
  @override
  final int exitCode;

  @override
  String toString() {
    return 'StepState.completed(instanceId: $instanceId, exitCode: $exitCode)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$StepState_CompletedImpl &&
            (identical(other.instanceId, instanceId) ||
                other.instanceId == instanceId) &&
            (identical(other.exitCode, exitCode) ||
                other.exitCode == exitCode));
  }

  @override
  int get hashCode => Object.hash(runtimeType, instanceId, exitCode);

  /// Create a copy of StepState
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$StepState_CompletedImplCopyWith<_$StepState_CompletedImpl> get copyWith =>
      __$$StepState_CompletedImplCopyWithImpl<_$StepState_CompletedImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() pending,
    required TResult Function(String instanceId) running,
    required TResult Function(String instanceId, int exitCode) completed,
    required TResult Function(String error) failed,
  }) {
    return completed(instanceId, exitCode);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? pending,
    TResult? Function(String instanceId)? running,
    TResult? Function(String instanceId, int exitCode)? completed,
    TResult? Function(String error)? failed,
  }) {
    return completed?.call(instanceId, exitCode);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? pending,
    TResult Function(String instanceId)? running,
    TResult Function(String instanceId, int exitCode)? completed,
    TResult Function(String error)? failed,
    required TResult orElse(),
  }) {
    if (completed != null) {
      return completed(instanceId, exitCode);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(StepState_Pending value) pending,
    required TResult Function(StepState_Running value) running,
    required TResult Function(StepState_Completed value) completed,
    required TResult Function(StepState_Failed value) failed,
  }) {
    return completed(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(StepState_Pending value)? pending,
    TResult? Function(StepState_Running value)? running,
    TResult? Function(StepState_Completed value)? completed,
    TResult? Function(StepState_Failed value)? failed,
  }) {
    return completed?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(StepState_Pending value)? pending,
    TResult Function(StepState_Running value)? running,
    TResult Function(StepState_Completed value)? completed,
    TResult Function(StepState_Failed value)? failed,
    required TResult orElse(),
  }) {
    if (completed != null) {
      return completed(this);
    }
    return orElse();
  }
}

abstract class StepState_Completed extends StepState {
  const factory StepState_Completed({
    required final String instanceId,
    required final int exitCode,
  }) = _$StepState_CompletedImpl;
  const StepState_Completed._() : super._();

  String get instanceId;
  int get exitCode;

  /// Create a copy of StepState
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$StepState_CompletedImplCopyWith<_$StepState_CompletedImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$StepState_FailedImplCopyWith<$Res> {
  factory _$$StepState_FailedImplCopyWith(
    _$StepState_FailedImpl value,
    $Res Function(_$StepState_FailedImpl) then,
  ) = __$$StepState_FailedImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String error});
}

/// @nodoc
class __$$StepState_FailedImplCopyWithImpl<$Res>
    extends _$StepStateCopyWithImpl<$Res, _$StepState_FailedImpl>
    implements _$$StepState_FailedImplCopyWith<$Res> {
  __$$StepState_FailedImplCopyWithImpl(
    _$StepState_FailedImpl _value,
    $Res Function(_$StepState_FailedImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of StepState
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? error = null}) {
    return _then(
      _$StepState_FailedImpl(
        error: null == error
            ? _value.error
            : error // ignore: cast_nullable_to_non_nullable
                  as String,
      ),
    );
  }
}

/// @nodoc

class _$StepState_FailedImpl extends StepState_Failed {
  const _$StepState_FailedImpl({required this.error}) : super._();

  @override
  final String error;

  @override
  String toString() {
    return 'StepState.failed(error: $error)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$StepState_FailedImpl &&
            (identical(other.error, error) || other.error == error));
  }

  @override
  int get hashCode => Object.hash(runtimeType, error);

  /// Create a copy of StepState
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$StepState_FailedImplCopyWith<_$StepState_FailedImpl> get copyWith =>
      __$$StepState_FailedImplCopyWithImpl<_$StepState_FailedImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() pending,
    required TResult Function(String instanceId) running,
    required TResult Function(String instanceId, int exitCode) completed,
    required TResult Function(String error) failed,
  }) {
    return failed(error);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? pending,
    TResult? Function(String instanceId)? running,
    TResult? Function(String instanceId, int exitCode)? completed,
    TResult? Function(String error)? failed,
  }) {
    return failed?.call(error);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? pending,
    TResult Function(String instanceId)? running,
    TResult Function(String instanceId, int exitCode)? completed,
    TResult Function(String error)? failed,
    required TResult orElse(),
  }) {
    if (failed != null) {
      return failed(error);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(StepState_Pending value) pending,
    required TResult Function(StepState_Running value) running,
    required TResult Function(StepState_Completed value) completed,
    required TResult Function(StepState_Failed value) failed,
  }) {
    return failed(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(StepState_Pending value)? pending,
    TResult? Function(StepState_Running value)? running,
    TResult? Function(StepState_Completed value)? completed,
    TResult? Function(StepState_Failed value)? failed,
  }) {
    return failed?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(StepState_Pending value)? pending,
    TResult Function(StepState_Running value)? running,
    TResult Function(StepState_Completed value)? completed,
    TResult Function(StepState_Failed value)? failed,
    required TResult orElse(),
  }) {
    if (failed != null) {
      return failed(this);
    }
    return orElse();
  }
}

abstract class StepState_Failed extends StepState {
  const factory StepState_Failed({required final String error}) =
      _$StepState_FailedImpl;
  const StepState_Failed._() : super._();

  String get error;

  /// Create a copy of StepState
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$StepState_FailedImplCopyWith<_$StepState_FailedImpl> get copyWith =>
      throw _privateConstructorUsedError;
}
