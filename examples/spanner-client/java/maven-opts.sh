# Apply JVM flags for Maven when it runs on JDK 24+.
# exec:java runs inside Maven's JVM, so MAVEN_OPTS is the supported knob.
#
# protobuf-java (pulled in by google-cloud-spanner) still calls sun.misc.Unsafe
# on JDK 24+ unless this flag is set (JEP 498).

if [[ -z "${SPANNER_JAVA_MAVEN_OPTS_APPLIED:-}" ]]; then
  _mvn_java_major="$(
    mvn -version 2>&1 | sed -n 's/.*Java version: \([0-9][0-9]*\).*/\1/p' | head -1
  )"
  if [[ -n "${_mvn_java_major}" && "${_mvn_java_major}" -ge 24 ]]; then
    case " ${MAVEN_OPTS:-} " in
      *" --sun-misc-unsafe-memory-access=allow "*) ;;
      *)
        MAVEN_OPTS="${MAVEN_OPTS:+$MAVEN_OPTS }--sun-misc-unsafe-memory-access=allow"
        export MAVEN_OPTS
        ;;
    esac
  fi
  export SPANNER_JAVA_MAVEN_OPTS_APPLIED=1
fi
